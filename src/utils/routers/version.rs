//! Return the latest Endless Void version number.
use crate::prelude::*;
use axum::extract::State;

/// Returns the version number of the latest
/// release of Endless Void, as set in the config.
pub async fn version(State(db): State<SharedAppState>) -> String {
    db.config().endless_void_version.clone()
}
