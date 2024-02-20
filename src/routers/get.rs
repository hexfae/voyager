use crate::server::SharedAppState;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use color_eyre::eyre::Result;
use std::net::SocketAddr;

/// Returns a list of levels stored in the database.
///
/// Returns a JSON array of JSON objects containing level metadata.
///
/// The format is as follows:
///
/// ```json
/// {
///     "name": String,
///     "data": String,
///     "author": String,
///     "author_brand": Number,
///     "burden": Number,
///     "upload_date": String
/// }
/// ```
///
/// Note: See [Level] for documentation about the keys.
///
/// Returns 200 OK and a JSON array of levels if
/// getting succeeded, or 500 INTERNAL SERVER ERROR
/// and JSON null if something went wrong server-side.
///
/// # Errors
/// Returns an error if the database is bad or something TODO: docs
pub async fn get(
    State(state): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<(StatusCode, String), StatusCode> {
    tracing::info!("GET sent by {}", addr.ip());
    Ok((StatusCode::OK, state.levels()))
}
