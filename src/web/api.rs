use axum::{extract::State, Json};

use crate::{model::Route, store::Store};

/// Return all routes from the store as JSON, sorted by (group, order, name).
pub async fn api_routes(State(store): State<Store>) -> Json<Vec<Route>> {
    Json(store.list())
}
