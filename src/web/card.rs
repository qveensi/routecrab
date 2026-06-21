use askama::Template;

use crate::{icons::icon_url, model::Route};

/// Shared card template used both by the full board (via index.html `{% include %}`)
/// and by SSE OOB fragments.
#[derive(Template)]
#[template(path = "_card.html")]
pub struct CardTemplate<'a> {
    pub route: &'a Route,
    pub icon_url: String,
}

impl<'a> CardTemplate<'a> {
    /// Resolve the route's icon CDN URL and return a ready-to-render template.
    pub fn for_route(route: &'a Route) -> Self {
        let icon_url = icon_url(&route.name, &route.icon);
        Self { route, icon_url }
    }
}
