//! Return the latest
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
//! version number.
//!
use crate::prelude::*;
use axum::extract::{ConnectInfo, State};
use std::net::SocketAddr;
use tracing::info;

/// Returns the version number of the
/// [latest release of Endless Void](https://github.com/Skirlez/void-stranger-endless-void/releases/latest),
/// as set in the config.
pub async fn version(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> String {
    info!("Version check sent by {}", addr.ip());
    db.config.read().latest_endless_void_version.clone().0
}
