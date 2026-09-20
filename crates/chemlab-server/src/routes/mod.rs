mod api;
mod frontend;

use crate::state::AppState;
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(api::router())
        .merge(frontend::router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
#[path = "routes_tests.rs"]
mod tests;
