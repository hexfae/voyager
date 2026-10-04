pub mod cluster;
pub mod sector;

use crate::nexus::Nexus;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use tracing::instrument;

/// Return the latest endless void version
#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "GET", path = "/voyager/version")
)]
pub async fn beacon(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
) -> Response {
    let latest_endless_void_version = nexus.manifest.read().latest_endless_void_version();
    (StatusCode::OK, latest_endless_void_version).into_response()
}
