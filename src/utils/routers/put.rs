//! Router for the PUT HTTP method.

use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::info;

/// Updates an already uploaded level in the database.
///
/// Takes in a level in Void Stranger Level (VSL) format and a
/// [ULID](https://github.com/ulid/spec) key.
/// The format is as follows:
///
/// `version|name|description|music|author|brand|burdens|tiles|objects|key`
///
/// Returns 201 CREATED if successful. Returns 400 BAD REQUEST on invalid
/// level data. Returns 401 UNAUTHORIZED on invalid key. Returns 404 NOT
/// FOUND on if somehow, the level data and key are valid, but the key is
/// not associated with any uploaded level.
pub async fn put(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    input: String,
) -> Result<StatusCode> {
    let addr = addr.ip();
    info!("PUT sent by {addr}: {input}");
    if db.ip_is_banned(&addr) {
        return Err(Error::Banned);
    }

    // TODO: improve
    let level = Level::new_from_put(&input, addr)?;
    let key = level.key;
    let mut parsed = level.into_parsed()?;

    let old_level = db.get(&key)?;
    parsed.set_dates_to_now();
    parsed.set_uploaded_from(old_level)?;
    let level = parsed.into_level();
    db.insert(level);
    info!("PUT success by {addr}.");
    Ok(StatusCode::OK)
}
