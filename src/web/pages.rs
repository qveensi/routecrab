use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use rust_embed::RustEmbed;
use serde::Deserialize;

use crate::{config::Config, icons::icon_for, model::Route, store::Store};

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

/// A route paired with its resolved icon SVG (if any).
type RouteWithIcon = (Route, Option<&'static str>);

/// Groups of routes for template rendering: (group_name, routes_with_icons).
type RouteGroups = Vec<(String, Vec<RouteWithIcon>)>;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    title: &'a str,
    query: &'a str,
    groups: RouteGroups,
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
    let routes: Vec<Route> = state
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

    // Resolve icons per route, then group.
    let routes_with_icons: Vec<RouteWithIcon> = routes
        .into_iter()
        .map(|r| {
            let svg = icon_for(&r.name, &r.icon);
            (r, svg)
        })
        .collect();

    // Already sorted by (group, order, name) from store.list(); just group.
    let groups = group_routes_with_icons(routes_with_icons);

    let tmpl = IndexTemplate {
        title: &state.title,
        query: params.q.trim(),
        groups,
    };

    tmpl.render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Partition a sorted (route, icon_svg) slice into `(group_name, items)` pairs.
/// The order of groups follows the first occurrence in the input slice.
fn group_routes_with_icons(mut items: Vec<RouteWithIcon>) -> RouteGroups {
    let mut groups: RouteGroups = Vec::new();
    for (route, icon_svg) in items.drain(..) {
        if let Some(last) = groups.last_mut() {
            if last.0 == route.group {
                last.1.push((route, icon_svg));
                continue;
            }
        }
        groups.push((route.group.clone(), vec![(route, icon_svg)]));
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
