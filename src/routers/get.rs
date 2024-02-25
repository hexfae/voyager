use crate::server::SharedAppState;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;

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
    tracing::info!("GET sent by {}", addr.ip());
    (StatusCode::OK, state.levels())
}
