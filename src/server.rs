use axum::{
    routing::{get, post},
    Router,
};
use color_eyre::{eyre::Context, Result};
use routers::{levels::levels, upload::upload};
use std::net::SocketAddr;
use surrealdb::{engine::remote::ws::Client, Surreal};

mod routers;

pub async fn start_voyager(db: Surreal<Client>) -> Result<()> {
    tracing::info!("Voyager is now listening on port 3000.");
    let router = create_router(db);
    serve_app(router).await?;
    Ok(())
}

fn create_router(db: Surreal<Client>) -> Router {
    Router::new()
        .route("/void_stranger", get(levels))
        .route("/void_stranger", post(upload))
        .with_state(db)
}

async fn serve_app(router: Router) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {addr}");
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context(format!("starting server on {addr}"))?;
    Ok(())
}
