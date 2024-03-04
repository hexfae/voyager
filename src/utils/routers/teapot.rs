//! Router for any unhandled HTTP methods. Easter egg.
use axum::{extract::ConnectInfo, http::StatusCode};
use std::net::SocketAddr;
use tracing::info;

/// Returned when Voyager is a teapot (when
/// an unhandled HTTP request is sent).
#[allow(clippy::unused_async)]
pub async fn teapot(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> StatusCode {
    info!("TEAPOT sent by {}", addr.ip());
    StatusCode::IM_A_TEAPOT
}
