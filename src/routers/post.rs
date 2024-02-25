use crate::{parser::Level, server::SharedAppState};
use anyhow::Result;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::{error, info, warn};
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
// allow missing errors because it's not
// really relevant for an axum project
#[allow(clippy::missing_errors_doc)]
pub async fn post(
    State(state): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<(StatusCode, String), StatusCode> {
    info!("POST sent by {}: {}", addr.ip(), level);

    let level = Level::from(level.as_bytes());
    let (_, parsed_level) = level.parse().map_err(|why| {
        warn!("level could not be prased: {why}");
        StatusCode::BAD_REQUEST
    })?;

    let id = Ulid::new();

    info!(
        "POST completed: level {} by {} created",
        parsed_level.name, parsed_level.author
    );
    state.insert(id, level);

    if let Err(why) = state.save() {
        error!("database could not be saved! {why}");
    };
    Ok((StatusCode::CREATED, id.to_string()))
}
