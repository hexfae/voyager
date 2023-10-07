use color_eyre::{eyre::Context, Result};
use std::env;
use surrealdb::{engine::local::Db, Surreal};

// allowed since database runs in memory
// in debug mode, speedb in release mode
// else it'll complain about unused imports
#[allow(unused_imports)]
use surrealdb::engine::local::{Mem, SpeeDb};

/// Creates/uses an embedded SpeeDb database in `$PWD/voyager.db`
/// in release mode or runs database in memory in debug mode.
pub async fn connect_to_database() -> Result<Surreal<Db>> {
    let path = get_database_path()?;
    let db = create_surreal_instance(&path).await?;
    Ok(db)
}

#[cfg(not(debug_assertions))]
async fn create_surreal_instance(database_path: &str) -> Result<Surreal<Db>> {
    let db: Surreal<Db> = Surreal::new::<SpeeDb>(database_path)
        .await
        .context(format!("starting database {}", database_path))?;
    db.use_ns("voyager").use_db("voyager").await?;
    Ok(db)
}

#[cfg(debug_assertions)]
async fn create_surreal_instance(_database_path: &str) -> Result<Surreal<Db>> {
    let db: Surreal<Db> = Surreal::new::<Mem>(()).await?;
    db.use_ns("voyager").use_db("voyager").await?;
    Ok(db)
}

fn get_database_path() -> Result<String> {
    let current_working_directory = env::current_dir().context("getting current directory")?;
    let database_path = current_working_directory
        .join("voyager.db")
        .to_string_lossy()
        .to_string();
    Ok(database_path)
}
