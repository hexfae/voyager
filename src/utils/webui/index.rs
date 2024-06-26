//! The front page of the Voyager Web UI.
//!
//! Most of these functions are called by htmx.

use crate::prelude::*;
use crate::utils::level::IndexLevel;
use askama_axum::{IntoResponse, Template};
use axum::extract::Path;
use axum::{extract::State, response::Html};

#[derive(Template)]
#[template(path = "index.html")]
/// Askama template for rendering the front page.
struct Index;

#[derive(Template)]
#[template(path = "levels.html")]
/// Askama template for rendering the table of levels.
struct Levels {
    /// The levels to be displayed.
    levels: Vec<IndexLevel>,
}

/// The main page of the Voyager Web UI.
pub async fn index(auth_session: AuthSession) -> impl IntoResponse {
    auth_session
        .user
        .map_or(Html(r"unauthorized").into_response(), |_| {
            Index.into_response()
        })
}

/// Displays all levels.
///
/// This is lazily loaded by htmx on page load.
pub async fn levels(
    auth_session: AuthSession,
    State(db): State<SharedAppState>,
) -> impl IntoResponse {
    return_levels(auth_session, &db)
}

/// IP bans a specific IP address.
///
/// This is called by htmx on IP ban.
pub async fn ban(
    auth_session: AuthSession,
    State(db): State<SharedAppState>,
    ip: Path<String>,
) -> impl IntoResponse {
    let _ = db.ban(&ip);
    return_levels(auth_session, &db)
}

/// Deletes a level from the database, if found.
///
/// This is called by htmx on level deletion.
pub async fn delete(State(db): State<SharedAppState>, key: Path<String>) -> impl IntoResponse {
    if let Ok(key) = &key.parse() {
        let _ = db.delete(key);
    }
}

/// Returns all levels in a `<tbody>`.
///
/// See the `levels.html` template for details.
fn return_levels(auth_session: AuthSession, db: &SharedAppState) -> impl IntoResponse {
    let levels = db.index_levels();
    auth_session
        .user
        .map_or(Html(r"unauthorized").into_response(), |_| {
            Levels { levels }.into_response()
        })
}
