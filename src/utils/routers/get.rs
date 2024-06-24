//! Routers for the GET HTTP method.

use crate::prelude::*;
//for documentation
#[allow(unused_imports)]
use crate::utils::level::Data;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
};
use std::{net::SocketAddr, str::FromStr};
use tracing::info;

/// Returns a comma-separated list of all levels stored in the database.
///
/// See [`Data`] for details on level format.
///
/// Returns 200 OK and a comma-separated list.
pub async fn get(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> String {
    info!(
        "GET sent by {}: sending {} levels",
        addr.ip(),
        db.levels_len()
    );
    db.levels()
}

// TODO: candidate for refactoring
/// Validates the existence of levels in the database.
///
/// On startup, Endless Void sends a GET request to `/voyager/:keys`,
/// where `keys` is a comma-separated list of level keys. Voyager then
/// returns a sequence of 0's and 1's according to the existence of the
/// levels matching those keys.
///
/// For example, for `key1,key2,key3,key4`, if `key1`, `key2`, and `key4`
/// are valid, but `key3` is not, Voyager will return 200 OK and `1101`.
/// If any key fails to parse, Voyager will instead return 400 BAD REQUEST.
///
/// Valid is defined as "a level with a key that exists in the database."
///
/// Returns 200 OK and a sequence of 0's and 1's, or 400 BAD REQUEST.
pub async fn levels_exist(
    Path(keys): Path<String>,
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<(StatusCode, String)> {
    let addr = addr.ip();
    let split_keys = keys.split(',').collect::<Vec<&str>>();
    let input_keys_len = split_keys.len();
    info!("GET levels check sent by {addr}: {input_keys_len} keys");
    let parsed_keys = split_keys
        .into_iter()
        .filter_map(|k| Key::from_str(k).ok())
        .collect::<Vec<Key>>();
    if parsed_keys.len() != input_keys_len {
        let invalid_keys_len = input_keys_len - parsed_keys.len();
        info!("GET levels check failed by {addr}! {invalid_keys_len} keys were invalid");
        // most probable error
        return Err(Error::InvalidKey(ulid::DecodeError::InvalidLength));
    }
    let found = parsed_keys
        .iter()
        .map(|key| u8::from(db.contains(key)).to_string())
        .collect::<Vec<String>>();
    let existing = found.join("");
    info!(
        "GET levels check success by {addr}: returned {} levels",
        found.len()
    );
    Ok((StatusCode::OK, existing))
}
