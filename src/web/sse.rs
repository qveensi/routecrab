use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use tokio::sync::broadcast::error::RecvError;

use crate::{store::Store, web::pages::render_board};

/// Coalesce window: bursts of changes (e.g. the watcher's initial list) collapse
/// into a single board re-render.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Render the current board wrapped as an htmx OOB innerHTML swap of `#board`.
/// innerHTML (not outerHTML) preserves the `#board` element and its live SSE
/// connection while replacing all groups/cards — so new, removed, moved,
/// re-sorted, and hidden routes are all reflected correctly.
fn board_event(store: &Store) -> Event {
    let board = render_board(store);
    let data = format!(r#"<div hx-swap-oob="innerHTML:#board">{board}</div>"#);
    Event::default().event("board-refresh").data(data)
}

/// Stream a full board refresh on connect, then a debounced refresh after any
/// store change.
pub async fn sse_handler(State(store): State<Store>) -> impl IntoResponse {
    let mut rx = store.subscribe();
    let stream = async_stream::stream! {
        // Initial paint so a freshly-connected client is immediately consistent.
        yield Ok::<Event, Infallible>(board_event(&store));

        while let Ok(_) | Err(RecvError::Lagged(_)) = rx.recv().await {
            // Drain further changes for DEBOUNCE, then emit once.
            let sleep = tokio::time::sleep(DEBOUNCE);
            tokio::pin!(sleep);
            let mut closed = false;
            loop {
                tokio::select! {
                    _ = &mut sleep => break,
                    r = rx.recv() => {
                        if matches!(r, Err(RecvError::Closed)) { closed = true; break; }
                    }
                }
            }
            yield Ok(board_event(&store));
            if closed { break; }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
