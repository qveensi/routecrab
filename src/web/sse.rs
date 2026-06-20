use askama::Template;
use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    store::{Change, Store},
    web::card::CardTemplate,
};

// ── SSE handler ───────────────────────────────────────────────────────────

/// Stream store changes as Server-Sent Events.
///
/// Event design:
/// - `route-update`: data = rendered `_card.html` fragment with
///   `hx-swap-oob="outerHTML:#route-{id}"` so htmx replaces the card in place.
/// - `route-remove`: data = minimal sentinel fragment with
///   `hx-swap-oob="delete:#route-{id}"` so htmx removes the card element.
///
/// The client subscribes via `hx-ext="sse" sse-connect="/events"` on the
/// board container; each card listens with `sse-swap="route-update"` — but
/// since we use OOB swaps the board container itself does not need to be the
/// swap target; the OOB attribute handles targeting directly.
pub async fn sse_handler(State(store): State<Store>) -> impl IntoResponse {
    let rx = store.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            // Lagged means we missed some messages; skip and continue streaming.
            Err(_) => None,

            Ok(Change::Upsert(route)) => {
                let html = CardTemplate::for_route(&route).render().unwrap_or_default();
                // Inject the OOB swap attribute into the outer <div> tag.
                // The card template root is `<div class="card" id="route-{id}">`;
                // we prepend the hx-swap-oob attribute so htmx targets it by id.
                let oob = format!(r#"hx-swap-oob="outerHTML:#route-{}""#, route.id);
                // Insert the oob attribute right after the opening `<div`
                let fragment = if let Some(pos) = html.find("<div") {
                    let after_div = pos + 4; // length of "<div"
                    format!("{}<div {} {}", &html[..pos], oob, &html[after_div..])
                } else {
                    html
                };

                Some(Ok::<Event, std::convert::Infallible>(
                    Event::default().event("route-update").data(fragment),
                ))
            }

            Ok(Change::Remove(id)) => {
                // Empty div with delete OOB swap removes the card from the DOM.
                let fragment = format!(r#"<div id="route-{}" hx-swap-oob="delete"></div>"#, id);
                Some(Ok::<Event, std::convert::Infallible>(
                    Event::default().event("route-remove").data(fragment),
                ))
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
