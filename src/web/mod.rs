pub mod api;
pub mod card;
pub mod pages;
pub mod sse;

use axum::{routing::get, Router};
use axum_prometheus::{metrics_exporter_prometheus::PrometheusHandle, PrometheusMetricLayer};

use crate::{config::Config, store::Store};

/// Build the main app router. The prometheus layer is created once in `main`
/// (single global recorder) and passed in.
///
/// Endpoints:
/// - GET /           → HTML dashboard board
/// - GET /healthz    → 200 "ok"
/// - GET /api/routes → JSON list of all routes (sorted)
/// - GET /assets/{*path} → static embedded assets (htmx, css)
pub fn router(
    store: Store,
    cfg: Config,
    prometheus_layer: PrometheusMetricLayer<'static>,
) -> Router {
    let page_state = pages::page_state(store.clone(), &cfg);
    Router::new()
        .route("/", get(pages::index))
        .route("/assets/{*path}", get(pages::static_handler))
        .with_state(page_state)
        .merge(
            Router::new()
                .route("/healthz", get(healthz))
                .route("/api/routes", get(api::api_routes))
                .route("/events", get(sse::sse_handler))
                .with_state(store),
        )
        .layer(prometheus_layer)
}

/// Standalone router that serves `GET /metrics` from the recorder handle.
pub fn metrics_router(handle: PrometheusHandle) -> Router {
    Router::new().route("/metrics", get(move || async move { handle.render() }))
}

async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::{config::Config, model::Route, store::Store, web::router};

    /// All assertions share one router instance to avoid registering the
    /// global Prometheus recorder more than once per process.
    #[tokio::test]
    async fn web_router_integration() {
        let store = Store::new();
        store.upsert(Route {
            id: "a".into(),
            name: "a".into(),
            ..Default::default()
        });
        store.upsert(Route {
            id: "test-route-1".into(),
            name: "my-awesome-service".into(),
            // no title set — display_title() falls back to name
            group: "production".into(),
            url: "https://example.com".into(),
            description: "Test description".into(),
            ..Default::default()
        });
        store.upsert(Route {
            id: "svc-1".into(),
            name: "alpha-service".into(),
            ..Default::default()
        });
        store.upsert(Route {
            id: "svc-2".into(),
            name: "beta-service".into(),
            ..Default::default()
        });
        store.upsert(Route {
            id: "hidden-1".into(),
            name: "hidden-service".into(),
            hidden: true,
            ..Default::default()
        });

        let (layer, _handle) = axum_prometheus::PrometheusMetricLayer::pair();
        let app = router(store.clone(), Config::default(), layer);

        // healthz
        let res = app
            .clone()
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "healthz must be 200");

        // api/routes
        let res = app
            .clone()
            .oneshot(Request::get("/api/routes").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "api/routes must be 200");

        // GET / returns html containing a route name
        let res = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "index must be 200");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);
        assert!(body_str.contains("<html"), "body must contain <html");
        assert!(
            body_str.contains("my-awesome-service"),
            "body must contain the route name"
        );
        assert!(
            !body_str.contains("hidden-service"),
            "hidden routes must not appear on the board"
        );

        // search query filters routes
        let res = app
            .clone()
            .oneshot(Request::get("/?q=alpha").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "filtered index must be 200");
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("alpha-service"),
            "alpha should be visible"
        );
        assert!(
            !body_str.contains("beta-service"),
            "beta should be filtered out"
        );

        // Board partial renders the seeded visible route and excludes hidden.
        let board = crate::web::pages::render_board(&store);
        assert!(
            board.contains("my-awesome-service"),
            "board must list visible route"
        );
        assert!(
            !board.contains("hidden-service"),
            "board must exclude hidden route"
        );

        // GET /events returns 200 text/event-stream
        let res = app
            .clone()
            .oneshot(Request::get("/events").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "/events must be 200");
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with("text/event-stream"),
            "/events must serve text/event-stream, got: {ct}"
        );
    }

    #[tokio::test]
    async fn sse_emits_board_refresh_on_change() {
        use crate::{model::Route, store::Store, web::pages::render_board};

        let store = Store::new();
        store.upsert(Route {
            id: "sse-frag-test".into(),
            name: "grafana".into(),
            ..Default::default()
        });

        let board = render_board(&store);
        let fragment = format!(r#"<div hx-swap-oob="innerHTML:#board">{board}</div>"#);

        assert!(
            fragment.contains(r#"hx-swap-oob="innerHTML:#board""#),
            "must target #board"
        );
        assert!(
            fragment.contains("route-sse-frag-test"),
            "must contain the route card id"
        );
        assert!(
            fragment.contains("<svg"),
            "grafana icon must resolve to inline SVG"
        );
    }
}
