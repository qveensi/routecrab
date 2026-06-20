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

/// Update custom Prometheus gauges from the current snapshot of routes.
///
/// Must be called after `web::router` has been built (which registers the
/// global `metrics` recorder via `PrometheusMetricLayer::pair()`).
pub fn update_route_gauges(routes: &[Route]) {
    let total = routes.len() as f64;
    metrics::gauge!("routecrab_routes_total").set(total);

    let mut healthy = 0u32;
    let mut degraded = 0u32;
    let mut unhealthy = 0u32;
    let mut unknown = 0u32;

    for r in routes {
        match r.health {
            HealthStatus::Healthy => healthy += 1,
            HealthStatus::Degraded => degraded += 1,
            HealthStatus::Unhealthy => unhealthy += 1,
            HealthStatus::Unknown => unknown += 1,
        }
    }

    metrics::gauge!("routecrab_routes_by_health", "status" => "healthy").set(healthy as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "degraded").set(degraded as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "unhealthy").set(unhealthy as f64);
    metrics::gauge!("routecrab_routes_by_health", "status" => "unknown").set(unknown as f64);
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
}
