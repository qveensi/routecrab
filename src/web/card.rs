use askama::Template;

use crate::{icons::icon_for, model::Route};

/// Shared card template used both by the full board (via index.html `{% include %}`)
/// and by SSE OOB fragments.
#[derive(Template)]
#[template(path = "_card.html")]
pub struct CardTemplate<'a> {
    pub route: &'a Route,
    pub icon_svg: Option<&'static str>,
}

impl<'a> CardTemplate<'a> {
    /// Resolve the route's icon and return a ready-to-render template.
    pub fn for_route(route: &'a Route) -> Self {
        let icon_svg = icon_for(&route.name, &route.icon);
        Self { route, icon_svg }
    }
}
