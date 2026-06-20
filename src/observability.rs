use crate::model::{HealthStatus, Route};

/// Initialise the global tracing subscriber.
///
/// Chooses a JSON layer when `format == "json"`, plain fmt otherwise.
/// Respects `RUST_LOG` for overrides; falls back to `level`.
/// Uses `try_init` so calling this more than once (e.g. in tests) is a no-op.
pub fn init_tracing(level: &str, format: &str) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    if format == "json" {
        let subscriber = Registry::default().with(filter).with(fmt::layer().json());
        // Ignore error: already initialised (e.g. in tests).
        let _ = tracing::subscriber::set_global_default(subscriber);
    } else {
        let subscriber = Registry::default().with(filter).with(fmt::layer());
        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}

/// Counts of routes by health, EXCLUDING hidden routes (hidden = not surfaced
/// anywhere: board, health, or metrics).
pub(crate) struct RouteCounts {
    pub total: u32,
    pub healthy: u32,
    pub degraded: u32,
    pub unhealthy: u32,
    pub unknown: u32,
}

pub(crate) fn count_routes(routes: &[Route]) -> RouteCounts {
    let mut c = RouteCounts {
        total: 0,
        healthy: 0,
        degraded: 0,
        unhealthy: 0,
        unknown: 0,
    };
    for r in routes.iter().filter(|r| !r.hidden) {
        c.total += 1;
        match r.health {
            HealthStatus::Healthy => c.healthy += 1,
            HealthStatus::Degraded => c.degraded += 1,
            HealthStatus::Unhealthy => c.unhealthy += 1,
            HealthStatus::Unknown => c.unknown += 1,
        }
    }
    c
}

/// Update custom Prometheus gauges from the current snapshot of routes.
///
/// Must be called after `web::router` has been built (which registers the
/// global `metrics` recorder via `PrometheusMetricLayer::pair()`).
pub fn update_route_gauges(routes: &[Route]) {
    let c = count_routes(routes);
    metrics::gauge!("routecrab_routes_total").set(c.total as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "healthy").set(c.healthy as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "degraded").set(c.degraded as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "unhealthy").set(c.unhealthy as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "unknown").set(c.unknown as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_tracing_does_not_panic() {
        // Calling init_tracing once must not panic.
        // try_init / set_global_default errors are swallowed, so calling it
        // again in a subsequent test is also safe.
        init_tracing("info", "json");
    }

    #[test]
    fn init_tracing_text_does_not_panic() {
        init_tracing("debug", "text");
    }

    #[test]
    fn count_routes_excludes_hidden() {
        use crate::model::{HealthStatus, Route};
        let routes = vec![
            Route {
                url: "http://a".into(),
                health: HealthStatus::Healthy,
                ..Default::default()
            },
            Route {
                url: "http://b".into(),
                health: HealthStatus::Unhealthy,
                hidden: true,
                ..Default::default()
            },
            Route {
                url: "http://c".into(),
                health: HealthStatus::Degraded,
                ..Default::default()
            },
        ];
        let c = count_routes(&routes);
        assert_eq!(c.total, 2);
        assert_eq!(c.healthy, 1);
        assert_eq!(c.unhealthy, 0);
        assert_eq!(c.degraded, 1);
        assert_eq!(c.unknown, 0);
    }
}
