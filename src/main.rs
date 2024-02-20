use color_eyre::Result;
use tracing::info;
use voyager::server::start_voyager;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    info!("Voyager is launching.");
    start_voyager().await
}
