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
    State(levels): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<StatusCode> {
    let addr = addr.ip();
    info!("PUT sent by {addr}: {level}");

    // TODO: improve
    let (level, key) = level.rsplit_once('|').ok_or(Error::InvalidStructure)?;
    let key = key.parse()?;
    let mut level = Level::from(level)?;

    let old_level = levels.get(&key);
    if old_level.is_none() {
        info!("PUT fail by {addr}; level not in database");
        return Err(Error::LevelNotFound);
    }
    // TODO: not clone
    level.update_edited(old_level.expect("should never happen").clone())?;
    levels.insert(key, level);
    info!("PUT success by {addr}.");
    Ok(StatusCode::OK)
}
