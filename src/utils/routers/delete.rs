//! Router for the DELETE HTTP method.

use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::info;

/// Deletes a stored level in the database.
///
/// Takes in a [ULID](https://github.com/ulid/spec) key.
///
/// Returns 204 NO CONTENT if successful. Returns 404 NOT FOUND if the
/// key has no matching level in the database. Returns 400 BAD REQEUST
/// on invalid key.
pub async fn delete(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    key: String,
) -> Result<StatusCode> {
    let addr = addr.ip();
    if db.ip_is_banned(&addr) {
        return Err(Error::Banned);
    }
    info!("DELETE sent by {addr}");
    db.delete(&key)
}
