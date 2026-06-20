pub mod api;
pub mod card;
pub mod pages;
pub mod sse;

use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;

use crate::{config::Config, store::Store};

/// Build the main axum Router.
///
/// Endpoints:
/// - GET /           → HTML dashboard board
/// - GET /healthz    → 200 "ok"
/// - GET /api/routes → JSON list of all routes (sorted)
/// - GET /metrics    → Prometheus text exposition
/// - GET /assets/*   → static embedded assets (htmx, css)
pub fn router(store: Store, cfg: Config) -> Router {
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let page_state = pages::page_state(store.clone(), &cfg);

    Router::new()
        .route("/", get(pages::index))
        .route("/assets/*path", get(pages::static_handler))
        .with_state(page_state)
        .merge(
            Router::new()
                .route("/healthz", get(healthz))
                .route("/api/routes", get(api::api_routes))
                .route("/events", get(sse::sse_handler))
                .route(
                    "/metrics",
                    get(move || async move { metric_handle.render() }),
                )
                .with_state(store),
        )
        .layer(prometheus_layer)
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

        let app = router(store, Config::default());

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

    /// Assert that upserting a route causes the broadcast channel to emit a Change::Upsert,
    /// which the SSE handler renders into a fragment containing the OOB swap marker
    /// and the route's element id.
    #[tokio::test]
    async fn sse_emits_card_fragment_on_upsert() {
        use askama::Template;

        use crate::{
            store::{Change, Store},
            web::card::CardTemplate,
        };

        let store = Store::new();
        let mut rx = store.subscribe();

        store.upsert(crate::model::Route {
            id: "sse-frag-test".into(),
            name: "grafana".into(),
            ..Default::default()
        });

        // Receive the broadcast change emitted by the upsert.
        let change = tokio::time::timeout(tokio::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for broadcast")
            .expect("broadcast channel error");

        let route = match change {
            Change::Upsert(r) => *r,
            _ => panic!("expected Upsert, got something else"),
        };

        // Render the card fragment the same way sse_handler does.
        let html = CardTemplate::for_route(&route)
            .render()
            .expect("template render failed");

        // Simulate the OOB attribute injection performed by sse_handler.
        let oob = format!(r#"hx-swap-oob="outerHTML:#route-{}""#, route.id);
        let fragment = if let Some(pos) = html.find("<div") {
            let after_div = pos + 4; // len("<div")
            format!("{}<div {} {}", &html[..pos], oob, &html[after_div..])
        } else {
            html.clone()
        };

        assert!(
            fragment.contains("hx-swap-oob"),
            "fragment must contain OOB swap marker"
        );
        assert!(
            fragment.contains("route-sse-frag-test"),
            "fragment must contain the route element id"
        );
        // grafana has a vendored SVG — verify the icon resolved.
        assert!(
            html.contains("<svg"),
            "rendered card must contain inline SVG for grafana, got: {html}"
        );
    }
}
