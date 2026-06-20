use std::time::Duration;

use crate::model::{HealthStatus, Route};

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
#[allow(dead_code)]
pub async fn run(store: crate::store::Store, cfg: crate::config::Config) {
    if !cfg.health_enabled {
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(cfg.health_timeout)
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
