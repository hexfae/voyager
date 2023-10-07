use axum::{
    routing::{any, get, post},
    Router,
};
use color_eyre::{eyre::Context, Result};
use std::net::SocketAddr;
use surrealdb::{engine::local::Db, Surreal};

mod routers;

/// Starts the Voyager server on port 3000.
pub async fn start_voyager(db: Surreal<Db>) -> Result<()> {
    tracing::info!("Voyager is now listening on port 3000.");
    let router = create_router(db);
    serve_app(router).await?;
    Ok(())
}

fn create_router(db: Surreal<Db>) -> Router {
    Router::new()
        .route("/void_stranger", get(routers::get::get))
        .route("/void_stranger", post(routers::post::post))
        .route("/void_stranger", any(routers::teapot::teapot))
        .with_state(db)
}

async fn serve_app(router: Router) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context(format!("starting server on {addr}"))?;
    Ok(())
}
