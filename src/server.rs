use crate::{parser::Level, routers};
use axum::{
    routing::{any, get, post, put},
    Router,
};
use color_eyre::Result;
use dashmap::DashMap;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;
use ulid::Ulid;

pub type SharedAppState = Arc<AppState>;

#[derive(Debug, Default)]
pub struct AppState {
    levels: DashMap<Ulid, Level>,
}

impl AppState {
    #[must_use]
    pub fn new() -> SharedAppState {
        Arc::new(Self {
            levels: DashMap::new(),
        })
    }

    /// TODO
    /// # Errors
    /// This function will return an error if given invalid data lol
    pub fn from(level: &str) -> Result<SharedAppState> {
        let levels = ron::from_str(level)?;
        Ok(Arc::new(Self { levels }))
    }

    pub fn insert(&self, key: Ulid, level: Level) {
        self.levels.insert(key, level);
    }

    #[must_use]
    pub fn levels(&self) -> String {
        self.levels
            .clone()
            .into_read_only()
            .values()
            .map(|level| level.data.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }
}

/// Starts the Voyager server on port 3000.
///
/// # Errors
/// Returns an error if the app could not be served.
pub async fn start_voyager() -> Result<()> {
    info!("Voyager is now listening on port 3000.");
    let router = create_router();
    serve_app(router).await
}

fn create_router() -> Router {
    let levels = AppState::new();
    Router::new()
        .route("/void_stranger", get(routers::get::get))
        .route("/void_stranger", post(routers::post::post))
        .route("/void_stranger", put(routers::put::put))
        .route("/void_stranger", any(routers::teapot::teapot))
        .with_state(levels)
}

async fn serve_app(app: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
