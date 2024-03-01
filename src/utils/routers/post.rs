use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::info;
use ulid::Ulid;

/// Uploads a level to the database (if valid) and returns the ULID key to it.
///
/// Takes in a level in Void Stranger Level (VSL) format. A
/// [ULID](https://github.com/ulid/spec) key is generated and returned,
/// used for future editing/deleting.
///
/// The format is as follows:
///
/// `version|name|description|music|author|brand|burdens|tiles|objects`
///
/// Returns 201 CREATED and a ULID key if successful. Returns 400 BAD REQUEST if
/// the level was invalid.
pub async fn post(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<(StatusCode, String)> {
    info!("POST sent by {}: {}", addr.ip(), level);

    // TODO: remove redundancy
    let level = Level::new(level);
    let mut parsed = level.into_parsed()?;
    parsed.set_dates_to_now();
    info!("POST completed:\n{parsed}");

    let level = parsed.into_level();
    let id = Ulid::new();

    db.insert(id, level);
    db.save();
    Ok((StatusCode::CREATED, id.to_string()))
}
