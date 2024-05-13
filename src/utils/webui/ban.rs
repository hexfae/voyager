//! IP ban someone.

use crate::prelude::*;
use axum::{
    extract::{Path, State},
    response::Redirect,
};

/// IP bans a specific IP address and redirects to the home page.
pub async fn ban(State(db): State<SharedAppState>, ip: Path<String>) -> Redirect {
    let _ = db.ban(&ip);
    Redirect::to("/voyager/webui")
}
