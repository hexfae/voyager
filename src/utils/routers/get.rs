use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::info;
use ulid::Ulid;

/// Returns a comma-separated list of all levels stored in the database.
///
/// The format is as follows:
///
/// `version|name|description|music|author|brand|burdens|tiles|objects`
///
/// Version is an integer (current version is 1). Name, description, music, and
/// author are Base64-encoded strings. Brand is a 36-bit number. Burdens is a 4-bit
/// number. Tiles and objects are level data, encoded using Endless Void's Black Hole
/// Format (BHF).
///
/// Returns 200 OK and a comma-separated list.
pub async fn get(
    State(state): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> (StatusCode, String) {
    info!("GET sent by {}", addr.ip());
    (StatusCode::OK, state.levels())
}

pub async fn levels_exist(
    Path(keys): Path<String>,
    State(state): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<(StatusCode, String)> {
    let addr = addr.ip();
    info!("GET levels check sent by {addr}");
    let split_keys = keys.split(',').collect::<Vec<&str>>();
    let keys = split_keys
        .iter()
        .filter_map(|key| key.parse::<Ulid>().ok())
        .collect::<Vec<Ulid>>();
    if keys.len() != split_keys.len() {
        info!("GET levels check failed by {addr}; one or more keys were invalid!");
        return Err(Error::LevelNotFound);
    }
    let found = keys
        .iter()
        .map(|key| i32::from(state.contains(key)).to_string())
        .collect::<Vec<String>>();
    let existing = found.join("");
    info!(
        "GET levels check success by {addr}; returned {} levels",
        found.len()
    );
    Ok((StatusCode::OK, existing))
}
