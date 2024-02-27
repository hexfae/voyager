use crate::server::SharedAppState;
use anyhow::Result;
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
    State(levels): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    key: String,
) -> Result<StatusCode, StatusCode> {
    let addr = addr.ip();
    info!("DELETE sent by {}", addr);
    let deleted = levels.delete(&key).map_err(|why| {
        info!("DELETE failed by {addr}; invalid key: {why}");
        StatusCode::BAD_REQUEST
    })?;
    if deleted {
        info!("DELETE success by {addr}");
        Ok(StatusCode::NO_CONTENT)
    } else {
        info!("DELETE failed by {addr}; level not found");
        Err(StatusCode::NOT_FOUND)
    }
}
