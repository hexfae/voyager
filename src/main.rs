use color_eyre::Result;
use database::connect_to_database;
use log::start_logging;
use server::start_voyager;

mod database;
mod log;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    start_logging();
    tracing::info!("Voyager is launching.");
    let database = connect_to_database().await?;
    start_voyager(database).await?;
    Ok(())
}
