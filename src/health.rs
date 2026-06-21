use std::net::IpAddr;
use std::time::Duration;

use crate::model::{HealthStatus, Route};

/// Reject probe targets that point at the host's own loopback, the cloud
/// metadata / link-local range (169.254.0.0/16, fe80::/10), or an unspecified
/// address. Probe targets come from untrusted `routecrab.io/health-*`
/// annotations, so without this a tenant could aim routecrab's HTTP client at
/// `169.254.169.254` (cloud metadata) or a localhost admin port.
///
/// Private ranges (10/172.16/192.168) are intentionally allowed — probing
/// internal cluster services is a supported use case.
fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_link_local() || v4.is_unspecified() || v4.is_broadcast()
        }
        // fe80::/10 link-local has no stable std predicate; match the prefix.
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Return true if a probe target must not be requested. Blocks unparseable
/// URLs, `localhost`, and IP-literal hosts in the loopback/link-local/
/// unspecified ranges. Hostnames that resolve to those ranges via DNS are a
/// known residual (no pre-connect resolution here); the common metadata-IP and
/// localhost-port SSRF vectors use literals and are covered.
pub fn is_blocked_target(target: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(target) else {
        return true;
    };
    let Some(host) = u.host_str() else {
        return true;
    };
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") {
        return true;
    }
    // host_str keeps IPv6 brackets; strip them before parsing.
    let h = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = h.parse::<IpAddr>() {
        return is_blocked_ip(&ip);
    }
    false
}

/// Classify a probe result into a HealthStatus.
///
/// `degraded_after` choice in `run()`: we pass `cfg.health_timeout / 2`.
/// A response that arrives but takes more than half the configured timeout is a
/// warning sign (slow upstream) — flagged as Degraded rather than Healthy —
/// giving an early warning before the service crosses the hard timeout boundary.
pub fn classify(status: Option<u16>, elapsed: Duration, degraded_after: Duration) -> HealthStatus {
    match status {
        None => HealthStatus::Unhealthy,
        Some(code) if code >= 400 => HealthStatus::Unhealthy,
        Some(_) if elapsed > degraded_after => HealthStatus::Degraded,
        Some(_) => HealthStatus::Healthy,
    }
}

/// Return true if a route should be probed.
/// A route is eligible when it has either a non-empty public URL or an
/// explicit health_url override (the internal-healthz use case), and
/// monitoring has not been disabled, and the route is not hidden.
/// Note: `health_path` alone with an empty `url` stays ineligible — the path
/// requires a base origin to resolve and `probe_target` would return "".
pub fn should_check(r: &Route) -> bool {
    !r.hidden && !r.monitor_disabled && (!r.url.is_empty() || !r.health_url.is_empty())
}

/// Resolve the URL to probe for a route.
/// Precedence: `health_url` (full override) > `health_path` (path on the
/// route URL's origin) > `url` (the public URL itself).
pub fn probe_target(r: &Route) -> String {
    if !r.health_url.is_empty() {
        return r.health_url.clone();
    }
    if !r.health_path.is_empty() {
        if let Ok(mut u) = reqwest::Url::parse(&r.url) {
            u.set_path(&r.health_path);
            u.set_query(None);
            return u.to_string();
        }
    }
    r.url.clone()
}

/// Run the health-check loop. Returns immediately when health checking is
/// disabled in config. Otherwise ticks on `cfg.health_interval`, probing
/// each eligible route with a HEAD request and storing the result.
pub async fn run(store: crate::store::Store, cfg: crate::config::Config) {
    if !cfg.health_enabled {
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(cfg.health_timeout)
        // Do not follow redirects: a probed URL must not be able to bounce the
        // client to an internal address (SSRF pivot). The immediate status is
        // what we classify.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build reqwest client");

    // Degraded threshold: half the probe timeout. A response slower than this
    // is worth flagging but has not yet crossed the hard timeout boundary.
    let degraded_after = cfg.health_timeout / 2;

    let mut interval = tokio::time::interval(cfg.health_interval);
    loop {
        interval.tick().await;

        let routes = store.list();
        for route in routes {
            if !should_check(&route) {
                continue;
            }

            let target = probe_target(&route);
            // Refuse SSRF-prone targets (metadata IP, loopback, link-local).
            if is_blocked_target(&target) {
                tracing::warn!(route = %route.id, target = %target, "blocked SSRF-prone probe target");
                store.set_health(&route.id, HealthStatus::Unknown);
                continue;
            }
            let start = tokio::time::Instant::now();
            let status_code = client
                .head(&target)
                .send()
                .await
                .ok()
                .map(|resp| resp.status().as_u16());
            let elapsed = start.elapsed();

            let health = classify(status_code, elapsed, degraded_after);
            store.set_health(&route.id, health);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_maps_codes() {
        use std::time::Duration;
        assert_eq!(
            classify(Some(200), Duration::from_millis(10), Duration::from_secs(2)),
            HealthStatus::Healthy
        );
        assert_eq!(
            classify(Some(500), Duration::from_millis(10), Duration::from_secs(2)),
            HealthStatus::Unhealthy
        );
        assert_eq!(
            classify(Some(200), Duration::from_secs(5), Duration::from_secs(2)),
            HealthStatus::Degraded
        );
        assert_eq!(
            classify(None, Duration::from_millis(10), Duration::from_secs(2)),
            HealthStatus::Unhealthy
        );
    }

    #[test]
    fn skips_empty_url_and_disabled() {
        assert!(!should_check(&Route {
            url: "".into(),
            ..Default::default()
        }));
        assert!(!should_check(&Route {
            url: "http://x".into(),
            monitor_disabled: true,
            ..Default::default()
        }));
        assert!(should_check(&Route {
            url: "http://x".into(),
            ..Default::default()
        }));
    }

    #[test]
    fn checks_health_url_only_route() {
        // Only health_url set, empty public url → still eligible.
        assert!(should_check(&Route {
            url: "".into(),
            health_url: "https://internal/healthz".into(),
            ..Default::default()
        }));
        // monitor disabled still wins.
        assert!(!should_check(&Route {
            url: "".into(),
            health_url: "https://internal/healthz".into(),
            monitor_disabled: true,
            ..Default::default()
        }));
        // empty url + only health_path → still skipped (no base origin).
        assert!(!should_check(&Route {
            url: "".into(),
            health_path: "/healthz".into(),
            ..Default::default()
        }));
    }

    #[test]
    fn hidden_routes_skip_check() {
        // A hidden route should NOT be probed, even with a valid URL.
        assert!(!should_check(&Route {
            url: "http://x".into(),
            hidden: true,
            ..Default::default()
        }));
        // A non-hidden route with a URL should be probed (baseline).
        assert!(should_check(&Route {
            url: "http://x".into(),
            ..Default::default()
        }));
    }

    #[test]
    fn blocks_ssrf_prone_targets() {
        // Cloud metadata, loopback, link-local, unspecified, localhost → blocked.
        for t in [
            "http://169.254.169.254/latest/meta-data/",
            "http://127.0.0.1:8080/",
            "https://localhost/healthz",
            "http://0.0.0.0/",
            "http://[::1]/",
            "http://[fe80::1]/",
            "not-a-url",
        ] {
            assert!(is_blocked_target(t), "{t} must be blocked");
        }
        // Public hosts and private cluster ranges (internal-healthz use case)
        // must stay allowed.
        for t in [
            "https://app.example.com/healthz",
            "http://10.0.0.5:9000/healthz",
            "http://192.168.1.10/health",
            "http://172.16.0.3/",
        ] {
            assert!(!is_blocked_target(t), "{t} must be allowed");
        }
    }

    #[test]
    fn probe_target_precedence() {
        // full override wins
        assert_eq!(
            probe_target(&Route {
                url: "https://app.example.com/".into(),
                health_url: "https://app.example.com:9000/healthz".into(),
                ..Default::default()
            }),
            "https://app.example.com:9000/healthz"
        );
        // path override rewrites the path on the same origin
        assert_eq!(
            probe_target(&Route {
                url: "https://app.example.com/dashboard".into(),
                health_path: "/healthz".into(),
                ..Default::default()
            }),
            "https://app.example.com/healthz"
        );
        // no override → url unchanged
        assert_eq!(
            probe_target(&Route {
                url: "https://app.example.com/".into(),
                ..Default::default()
            }),
            "https://app.example.com/"
        );
    }
}
