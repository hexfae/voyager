use anyhow::Result;
use tracing::info;
use voyager::server::start_voyager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Voyager is launching.");
    start_voyager().await
}
