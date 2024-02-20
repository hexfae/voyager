use crate::{parser::Level, server::SharedAppState};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use color_eyre::Result;
use std::net::SocketAddr;
use tracing::{info, warn};
use ulid::Ulid;

/// Uploads a level to the database (if valid) and returns the key to it.
///
/// Takes in a JSON object containing level metadata. Then, creates a new
/// ULID and gets the current date and time, then uploads this information
/// to the database, along with returning the [ULID](https://github.com/ulid/spec), used for editing/deleting.
///
/// The format is as follows:
///
/// ```json
/// {
///     "name": String,
///     "data": String,
///     "author": String,
///     "author_brand": Number,
///     "inputs": String,
///     "burdens": Number
/// }
/// ```
///
/// Note: See [Level] for documentation about the keys.
///
/// ```json
/// {
///     "key": String
/// }
/// ```
///
/// Returns 201 CREATED and a JSON object containing a ULID if created,
/// 400 BAD REQUEST and a JSON null if the body is wrongly formatted, or
/// 500 INTERNAL SERVER ERROR and JSON null if something else went wrong.
///
/// # Errors
/// Returns HTTP 500 INTERNAL SERVER ERROR if the database could not be
/// accessed, or HTTP TODO:what if invalid level data is received.
pub async fn post(
    State(state): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<(StatusCode, String), StatusCode> {
    info!("POST sent by {}: {}", addr.ip(), level);

    let (_, level) = Level::from(level.as_bytes()).map_err(|why| {
        warn!("level could not be parsed: {why}");
        StatusCode::BAD_REQUEST
    })?;

    let id = Ulid::new();

    info!(
        "POST completed: level {} by {} created",
        level.name, level.author
    );
    state.insert(id, level);
    Ok((StatusCode::CREATED, id.to_string()))
}
