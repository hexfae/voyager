//! Voyage the Void.

mod error;
mod incantations;
mod nexus;

use std::net::SocketAddr;

use axum::{
    Router,
    routing::{get, post},
};
use error::Error;
use incantations::{adopt, amend, beacon, expunge, inscribe, probe, unveil};
use nexus::Nexus;
use tokio::net::TcpListener;

const ADDRESS: &str = "0.0.0.0:3000";

#[tokio::main]
async fn main() -> miette::Result<()> {
    tracing_subscriber::fmt::init();
    let nexus = Nexus::try_load()?;
    let app = Router::new()
        .route(
            "/voyager",
            get(unveil).post(inscribe).put(amend).delete(expunge),
        )
        .route("/voyager/version", get(beacon))
        .route("/voyager/orphanage", post(adopt))
        .route("/voyager/{keys}", get(probe))
        .with_state(nexus)
        .into_make_service_with_connect_info::<SocketAddr>();
    let listener = TcpListener::bind(ADDRESS)
        .await
        .map_err(|why| Error::Bind {
            address: ADDRESS.into(),
            source: why,
        })?;
    axum::serve(listener, app)
        .await
        .map_err(|why| Error::Serve { source: why })?;
    Ok(())
}
