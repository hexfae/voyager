use color_eyre::{eyre::Context, Result};
use std::env;
use surrealdb::{
    engine::local::{Db, SpeeDb},
    Surreal,
};

/// Creates/uses an embedded SpeeDb database in `$PWD/voyager.db`.
pub async fn connect_to_database() -> Result<Surreal<Db>> {
    let path = get_database_path()?;
    let db = create_surreal_instance(&path).await?;
    select_namespace_and_database(&db).await?;
    Ok(db)
}

async fn create_surreal_instance(database_path: &str) -> Result<Surreal<Db>> {
    tracing::debug!("starting database {}", database_path);
    let db: Surreal<Db> = Surreal::new::<SpeeDb>(database_path)
        .await
        .context(format!("starting database {}", database_path))?;
    Ok(db)
}

async fn select_namespace_and_database(db: &Surreal<Db>) -> Result<()> {
    tracing::debug!("using namespace and database voyager");
    db.use_ns("voyager")
        .use_db("voyager")
        .await
        .context("selecting namespace voyager, database voyager")?;
    Ok(())
}

fn get_database_path() -> Result<String> {
    let current_working_directory = env::current_dir()
        .context("getting current directory")?
        .to_string_lossy()
        .to_string();
    let database_path = format!("{current_working_directory}/voyager.db");
    Ok(database_path)
}
