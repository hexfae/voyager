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
    info!("DELETE sent by {addr} for {key}");
    if db.ip_is_banned(&addr) {
        info!("{addr} is banned");
        return Err(Error::Banned);
    }
    let key = key.parse()?;
    info!("Deleting level {key}...");
    db.delete(&key)
}
