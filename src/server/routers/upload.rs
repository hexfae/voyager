use axum::{extract::State, http::StatusCode, Json};
use surrealdb::{engine::remote::ws::Client, Surreal};
use voyager::{
    level::{metadata::Key, CreateLevel},
    Record,
};

pub async fn upload(
    State(db): State<Surreal<Client>>,
    Json(create_level): Json<CreateLevel>,
) -> (StatusCode, Json<Option<Key>>) {
    tracing::info!(
        "POST: Level \"{}\" by \"{}\".",
        create_level.level.name,
        create_level.level.author
    );
    let level = create_level.to_private_level();

    let create: Result<Vec<Record>, surrealdb::Error> = db.create("level").content(&level).await;
    match create {
        Ok(record) => {
            tracing::debug!("stored level in database: {record:?}");
            (StatusCode::CREATED, Json(Some(level.key)))
        }
        Err(why) => {
            tracing::warn!("could not store level in database: {why}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
        }
    }
}
