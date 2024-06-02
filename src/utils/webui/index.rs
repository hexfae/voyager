//! The front page of the Voyager Web UI.

use askama_axum::{IntoResponse, Template};
use axum::{extract::State, response::Html};

use crate::prelude::*;
use crate::utils::level::IndexLevel;

#[derive(Template)]
#[template(path = "index.html")]
/// Askama template for rendering the front page.
struct Index {
    /// All stored levels.
    levels: Vec<IndexLevel>,
}

pub async fn index(
    auth_session: AuthSession,
    State(db): State<SharedAppState>,
) -> impl IntoResponse {
    let levels = db.index_levels();
    auth_session
        .user
        .map_or(Html(r"unauthorized").into_response(), |_| {
            Index { levels }.into_response()
        })
}
