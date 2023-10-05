use axum::{extract::State, http::StatusCode, Json};
use surrealdb::{engine::local::Db, Surreal};
use voyager::level::{Level, PrivateLevel};

/// Returns a status code (`200 OK` or `500 INTERNAL_SERVER_ERROR`)
/// along with a JSON array containing level objects. Format TBD.
#[tracing::instrument]
pub async fn levels(State(db): State<Surreal<Db>>) -> (StatusCode, Json<Option<Vec<Level>>>) {
    tracing::info!("GET: Level list.");
    let select: Result<Vec<PrivateLevel>, surrealdb::Error> = db.select("level").await;

    let levels = match select {
        Ok(levels) => {
            tracing::debug!("got list of levels");
            levels
        }
        Err(why) => {
            tracing::warn!("Could not get list of levels: {why}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(None));
        }
    };

    let public_levels = levels.into_iter().map(|m| m.to_level()).collect();

    (StatusCode::OK, Json(Some(public_levels)))
}
