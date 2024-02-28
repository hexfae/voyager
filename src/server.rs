use crate::{parser::Level, routers};
use anyhow::Result;
use axum::{
    routing::{any, delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    fs::{read, read_to_string, write},
    net::SocketAddr,
    sync::Arc,
};
use tracing::{info, warn};
use ulid::Ulid;

pub type SharedAppState = Arc<AppState>;

#[derive(Debug, Serialize, Deserialize)]
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

    /// # Panics
    /// Panics if a database if found, but deserializing it fails.
    #[must_use]
    pub fn load() -> SharedAppState {
        let input = read("voyager.db");
        input.map_or_else(
            |_| {
                info!("Existing database not found!");
                Self::new()
            },
            |level| {
                info!("Existing database found!");
                Self::from(&level).expect("valid database file")
            },
        )
    }

    /// # Errors
    /// Errors I/O TODO: write
    pub fn save(&self) -> Result<()> {
        let bytes = bincode::serialize(&self)?;
        write("voyager.db", bytes)?;
        Ok(())
    }

    /// TODO
    /// # Errors
    /// This function will return an error if given invalid data lol
    pub fn from(level: &[u8]) -> Result<SharedAppState> {
        let levels = bincode::deserialize(level)?;
        Ok(Arc::new(Self { levels }))
    }

    pub fn insert(&self, key: Ulid, level: Level) {
        self.levels.insert(key, level);
    }

    #[must_use]
    pub fn contains(&self, input: &Ulid) -> bool {
        self.levels.contains_key(input)
    }

    pub fn delete(&self, input: &str) -> Result<bool> {
        let key = input.parse::<Ulid>()?;
        if self.levels.remove(&key).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[must_use]
    pub fn levels(&self) -> String {
        self.levels
            .clone()
            .into_read_only()
            .values()
            // TODO: can i return &[u8] for GET instead?
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
    let levels = AppState::load();
    Router::new()
        .route("/voyager", get(routers::get::get))
        .route("/voyager/:key", get(routers::get::levels_exist))
        .route("/voyager", post(routers::post::post))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
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
