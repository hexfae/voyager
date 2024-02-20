use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};

use crate::server::SharedAppState;

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
/// This returns an error when an invalid key is given.
// TODO: this entire function
#[allow(dead_code, clippy::unused_async, unused_variables)]
pub async fn put(
    // State(db): State<Surreal<Client>>,
    State(levels): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<StatusCode, StatusCode> {
    tracing::debug!("PUT sent by {}: {}", addr.ip(), level);
    // let level = serde_json::from_str::<Level>(&body);

    // let new_level = match level {
    //     Ok(level) => level,
    //     Err(why) => {
    //         tracing::warn!("PUT FAIL: {why}");
    //         return Err(StatusCode::BAD_REQUEST);
    //     }
    // };

    tracing::info!("PUT SUCCESS.");

    // let select: Result<Option<Level>, surrealdb::Error> =
    //     db.select(("level", edit_level.key.to_string())).await;

    // let mut old_level = match select {
    //     Err(why) => {
    //         tracing::warn!("could not get level in database: {why}");
    //         return Err(StatusCode::INTERNAL_SERVER_ERROR);
    //     }
    //     Ok(option) => match option {
    //         Some(level) => level,
    //         None => {
    //             tracing::warn!("could not store level in database");
    //             return Err(StatusCode::INTERNAL_SERVER_ERROR);
    //         }
    //     },
    // };

    // let key = edit_level.key;

    // old_level.edit(edit_level);

    // let update: Result<Option<Level>, surrealdb::Error> = db
    //     .update(("level", key.to_string()))
    //     .content(&old_level)
    //     .await;

    // match update {
    //     Err(why) => {
    //         tracing::warn!("could not store level in database: {why}");
    //         Err(StatusCode::INTERNAL_SERVER_ERROR)
    //     }
    //     Ok(option) => match option {
    //         Some(level) => {
    //             tracing::debug!("stored level in database: {level:?}");
    //             Ok(StatusCode::CREATED)
    //         }
    //         None => {
    //             tracing::warn!("could not store level in database");
    //             Err(StatusCode::INTERNAL_SERVER_ERROR)
    //         }
    //     },
    // }
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}
