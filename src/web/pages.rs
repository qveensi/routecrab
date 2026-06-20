use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use rust_embed::RustEmbed;
use serde::Deserialize;

use crate::{config::Config, model::Route, store::Store};

// ── Embedded static assets ────────────────────────────────────────────────

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

/// Serve a file embedded from the `assets/` directory.
pub async fn static_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_type(&path);
            (
                [(axum::http::header::CONTENT_TYPE, mime)],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, b"not found".to_vec()).into_response(),
    }
}

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".html") || path.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

// ── Index template ────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    title: &'a str,
    query: &'a str,
    groups: Vec<(String, Vec<Route>)>,
}

// ── Query params ──────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct BoardQuery {
    #[serde(default)]
    pub q: String,
}

// ── App state shared between pages ────────────────────────────────────────

#[derive(Clone)]
pub struct PageState {
    pub store: Store,
    pub title: String,
}

// ── Index handler ─────────────────────────────────────────────────────────

pub async fn index(
    State(state): State<PageState>,
    Query(params): Query<BoardQuery>,
) -> Result<Html<String>, StatusCode> {
    let q = params.q.trim().to_lowercase();
    let mut routes: Vec<Route> = state
        .store
        .list()
        .into_iter()
        .filter(|r| !r.hidden)
        .filter(|r| {
            if q.is_empty() {
                return true;
            }
            r.name.to_lowercase().contains(&q)
                || r.display_title().to_lowercase().contains(&q)
                || r.description.to_lowercase().contains(&q)
        })
        .collect();

    // Already sorted by (group, order, name) from store.list(); just group.
    let groups = group_routes(&mut routes);

    let tmpl = IndexTemplate {
        title: &state.title,
        query: params.q.trim(),
        groups,
    };

    tmpl.render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Partition a sorted route slice into `(group_name, routes)` pairs.
/// The order of groups follows the first occurrence in the input slice.
fn group_routes(routes: &mut Vec<Route>) -> Vec<(String, Vec<Route>)> {
    let mut groups: Vec<(String, Vec<Route>)> = Vec::new();
    for route in routes.drain(..) {
        if let Some(last) = groups.last_mut() {
            if last.0 == route.group {
                last.1.push(route);
                continue;
            }
        }
        groups.push((route.group.clone(), vec![route]));
    }
    groups
}

/// Build a PageState from store + config — used by the router.
pub fn page_state(store: Store, cfg: &Config) -> PageState {
    PageState {
        store,
        title: cfg.title.clone(),
    }
}
