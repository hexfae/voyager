use crate::{parser::Level, server::SharedAppState};
use anyhow::Result;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::{debug, info};

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
    // State(db): State<Surreal<Client>>,
    State(levels): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<StatusCode, StatusCode> {
    let addr = addr.ip();
    info!("PUT sent by {addr}");
    debug!("PUT sent by {addr}: {level}");

    // TODO: improve
    let (level, key) = level.rsplit_once('|').ok_or_else(|| {
        info!("PUT failed by {addr}; input was too small");
        StatusCode::BAD_REQUEST
    })?;
    let key = key.parse().map_err(|why| {
        info!("PUT failed by {addr}; invalid key: {why}");
        StatusCode::UNAUTHORIZED
    })?;
    // no need for the parsed level,
    // but parsing validates it
    Level::parse(level).map_err(|why| {
        info!("PUT failed by {addr}; invalid level data: {why}");
        StatusCode::BAD_REQUEST
    })?;
    // TODO: fix redundancy below
    let mut level = Level::from(level).map_err(|why| {
        info!("PUT failed by {addr}; invalid level data: {why}");
        StatusCode::BAD_REQUEST
    })?;

    if levels.contains(&key) {
        level.update_edited();
        levels.insert(key, level);
        info!("PUT success by {addr}.");
        Ok(StatusCode::OK)
    } else {
        info!("PUT fail by {addr}; level not in database");
        Err(StatusCode::NOT_FOUND)
    }
}
