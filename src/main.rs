mod error;
mod prelude;
mod utils;

#[tokio::main]
async fn main() -> prelude::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Voyager is launching.");
    utils::server::start_voyager().await
}
