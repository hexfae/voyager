use axum::{extract::State, http::StatusCode, Json};
use surrealdb::{engine::local::Db, Surreal};
use ulid::Ulid;
use voyager::level::{CreateLevel, PrivateLevel};

/// Creates a level in the database, then returns
/// a status code (`200 OK` or `500 INTERNAL_SERVER_ERROR`)
/// along with the level's ID/key if created.
pub async fn upload(
    State(db): State<Surreal<Db>>,
    Json(create_level): Json<CreateLevel>,
) -> (StatusCode, Json<Option<Ulid>>) {
    tracing::info!(
        "POST: Level \"{}\" by \"{}\".",
        create_level.level.name,
        create_level.level.author
    );
    let level = create_level.to_private_level();

    println!("going to create");

    let id = Ulid::new();

    let create: Result<Option<PrivateLevel>, surrealdb::Error> =
        db.create(("level", id.to_string())).content(&level).await;
    println!("created");
    match create {
        Ok(option) => match option {
            Some(level) => {
                tracing::debug!("stored level in database: {level:?}");
                (StatusCode::CREATED, Json(Some(id)))
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
