//! Contains [`AppState`], related methods, and
//! various Axum server-related functions.
use super::routers;
use crate::prelude::*;
use axum::{
    http::StatusCode,
    routing::{any, delete, get, post, put},
    Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    fs::{read, write},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};
use ulid::Ulid;

// for documentation
#[allow(unused_imports)]
use crate::utils::routers::post::orphanage;

/// Thread-safe app state, used across Voyager.
pub type SharedAppState = Arc<AppState>;

/// Poor man's database. Two [`DashMap`]s
/// of levels and orphans respectively.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    /// Every key and its matching uploaded, validated level.
    levels: DashMap<Ulid, Level>,
    /// Every key and its matching validated orphan (see [`orphanage`]).
    orphans: DashMap<Ulid, Level>,
}

impl AppState {
    /// Creates a new, empty Voyager database.
    #[must_use]
    fn new() -> SharedAppState {
        Arc::new(Self {
            levels: DashMap::new(),
            orphans: DashMap::new(),
        })
    }

    /// Attempts to load a Voyager database from
    /// `./voyager.db`. If it fails (likely due
    /// to it not yet existing), it instead creates
    /// a new one using `Self::new()`;
    ///
    /// # Panics
    /// Panics if a Voyager database is found, but
    /// deserializing it fails. Most likely, some
    /// data structure had a breaking change (or
    /// the file is corrupted).
    #[must_use]
    fn load() -> SharedAppState {
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

    /// Attempts to save itself to `./voyager.db`.
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    fn save(&self) {
        match bincode::serialize(&self) {
            Ok(bytes) => {
                if let Err(why) = write("voyager.db", bytes) {
                    warn!("database could not be saved: {why}");
                };
            }
            Err(why) => warn!("database could not be serialized: {why}"),
        }
    }

    /// Attempts to deserialize a Voyager database
    /// from bytes.

    /// # Errors
    /// This function will return an error if
    /// deserializing it fails. Most likely, some
    /// data structure had a breaking change (or
    /// the file is corrupted).
    fn from(level: &[u8]) -> Result<SharedAppState> {
        let levels = bincode::deserialize(level)?;
        Ok(Arc::new(levels))
    }

    /// Inserts a level and its key and saves to a file.
    pub fn insert(&self, key: Ulid, level: Level) {
        self.levels.insert(key, level);
        self.save();
    }

    /// Inserts an orphan and its key and saves to a file.
    pub fn insert_orphan(&self, key: Ulid, level: Level) {
        self.orphans.insert(key, level);
        self.save();
    }

    /// Checks if the database contains the specified key.
    #[must_use]
    pub fn contains(&self, input: &Ulid) -> bool {
        self.levels.contains_key(input)
    }

    /// Moves a level and its key from the orphans list
    /// to the levels list, if found.
    pub fn adopt_orphan(&self, input: &Ulid) -> Result<()> {
        let (level, key) = self.orphans.remove(input).ok_or(Error::LevelNotFound)?;
        self.insert(level, key);
        Ok(())
    }

    /// Get a reference to a level in the database, if it exists.
    #[must_use]
    pub fn get(&self, input: &Ulid) -> Option<dashmap::mapref::one::Ref<'_, ulid::Ulid, Level>> {
        self.levels.get(input)
    }

    /// Deletes a level from the database, if it exists.
    pub fn delete(&self, input: &str) -> Result<StatusCode> {
        let key = input.parse::<Ulid>()?;
        if self.levels.remove(&key).is_some() {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(Error::LevelNotFound)
        }
    }

    /// Returns a comma-separated lists of all stored levels.
    ///
    /// See [`Data`] for details on level format.
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

/// Creates a new [`Router`] for Voyager.
fn create_router() -> Router {
    let levels = AppState::load();
    Router::new()
        .route("/voyager", get(routers::get::get))
        .route("/voyager/:keys", get(routers::get::levels_exist))
        .route("/voyager", post(routers::post::post))
        .route("/voyager/orphanage", post(routers::post::orphanage))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
        .with_state(levels)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
}

/// Serves the Voyager app on port 3000.
async fn serve_app(app: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

/// Function necessary for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
