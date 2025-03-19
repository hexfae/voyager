//! The front page of the Void Voyager Web UI.
//!
//! Most of these functions are called by htmx.

use crate::nexus::Nexus;
use crate::webui::backend::Backend;
use askama_axum::{IntoResponse, Template};
use axum::extract::Path;
use axum::extract::State;
use axum_login::AuthSession;
use void_codex::Sector;

#[derive(Template)]
#[template(path = "index.html")]
/// Askama template for rendering the front page.
struct Index;

#[derive(Template)]
#[template(path = "levels.html")]
/// Askama template for rendering the table of levels.
struct Levels {
    /// The levels to be displayed.
    levels: Vec<Sector>,
}

#[derive(Template)]
#[template(path = "unauthorized.html")]
struct Unauthorized;

/// The main page of the Void Voyager Web UI.
pub async fn index(auth_session: AuthSession<Backend>) -> askama_axum::Response {
    auth_session.user.map_or_else(
        || askama_axum::into_response(&Unauthorized),
        |_| askama_axum::into_response(&Index),
    )
}

/// Displays all levels.
///
/// This is lazily loaded by htmx on page load.
pub async fn levels(
    auth_session: AuthSession<Backend>,
    State(nexus): State<Nexus>,
) -> askama_axum::Response {
    return_levels(auth_session, &nexus)
}

/// IP bans a specific IP address.
///
/// This is called by htmx on IP ban.
pub async fn ban(
    auth_session: AuthSession<Backend>,
    State(nexus): State<Nexus>,
    origin: Path<String>,
) -> askama_axum::Response {
    if let Ok(origin) = origin.parse() {
        nexus.vaporize(origin);
    }
    return_levels(auth_session, &nexus)
}

/// Deletes a level from the database, if found.
///
/// This is called by htmx on level deletion.
pub async fn delete(State(nexus): State<Nexus>, sigil: Path<String>) {
    nexus.atlas.expunge(sigil.to_string());
}

/// Returns all levels in a `<tbody>`.
///
/// See the `levels.html` template for details.
fn return_levels(auth_session: AuthSession<Backend>, nexus: &Nexus) -> askama_axum::Response {
    let levels = nexus.atlas.sectors();
    auth_session.user.map_or_else(
        || Unauthorized.into_response(),
        |_| Levels { levels }.into_response(),
    )
}
