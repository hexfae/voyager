use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use surrealdb::{engine::local::Db, Surreal};
use ulid::Ulid;
use voyager::Level;

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
pub async fn post(State(db): State<Surreal<Db>>, body: String) -> (StatusCode, Json<Option<Key>>) {
    tracing::debug!("POST received: {body}");
    let level = serde_json::from_str::<Level>(&body);

    let mut level = match level {
        Err(why) => {
            tracing::warn!("POST FAIL: {why}");
            return (StatusCode::BAD_REQUEST, Json(None));
        }
        Ok(level) => level,
    };

    tracing::info!(
        "POST SUCCESS: Level \"{}\" by \"{}\".",
        level.name,
        level.author
    );

    let id = Ulid::new();

    level.upload_date = Some(chrono::Utc::now());

    let create: Result<Option<Level>, surrealdb::Error> =
        db.create(("level", id.to_string())).content(&level).await;

    match create {
        Ok(option) => match option {
            Some(level) => {
                tracing::debug!("stored level in database: {level:?}");
                (StatusCode::CREATED, Json(Some(Key { key: id })))
            }
            None => {
                tracing::warn!("could not store level in database");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
            }
        },
        Err(why) => {
            tracing::warn!("could not store level in database: {why}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
        }
    }
}

/// Used to easily return the ULID that gets created
/// upon level upload inside of a JSON object.
#[derive(Serialize)]
pub struct Key {
    key: Ulid,
}
