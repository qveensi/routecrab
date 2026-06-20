pub mod api;

use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;

use crate::{config::Config, store::Store};

/// Build the main axum Router.
///
/// Endpoints:
/// - GET /healthz       → 200 "ok"
/// - GET /api/routes    → JSON list of all routes (sorted)
/// - GET /metrics       → Prometheus text exposition
///
/// The router is intentionally flat and extensible: callers (tasks 8/9) can
/// merge additional routers via `Router::merge` before binding.
pub fn router(store: Store, _cfg: Config) -> Router {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/routes", get(api::api_routes))
        .route(
            "/metrics",
            get(move || async move { metric_handle.render() }),
        )
        .with_state(store)
        .layer(prometheus_layer)
}

async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::{config::Config, model::Route, store::Store, web::router};

    #[tokio::test]
    async fn healthz_ok_and_api_lists() {
        let store = Store::new();
        store.upsert(Route {
            id: "a".into(),
            name: "a".into(),
            ..Default::default()
        });
        let app = router(store, Config::default());
        let res = app
            .clone()
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let res = app
            .oneshot(Request::get("/api/routes").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
    }
}
