use axum::{extract::State, http::StatusCode, Json};
use surrealdb::{engine::local::Db, Surreal};
use voyager::Level;

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
pub async fn get(State(db): State<Surreal<Db>>) -> (StatusCode, Json<Option<Vec<Level>>>) {
    tracing::info!("GET: Level list.");
    let select: Result<Vec<Level>, surrealdb::Error> = db.select("level").await;

    let mut levels = match select {
        Ok(levels) => {
            tracing::debug!("got list of levels");
            levels
        }
        Err(why) => {
            tracing::warn!("Could not get list of levels: {why}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(None));
        }
    };

    levels.iter_mut().for_each(|l| l.to_public());

    (StatusCode::OK, Json(Some(levels)))
}
