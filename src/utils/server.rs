//! Contains [`AppState`], related methods, and
//! various Axum server-related functions.
use crate::prelude::*;
use crate::utils::{level::Validated, routers, webui};
use axum::{
    async_trait,
    http::StatusCode,
    routing::{any, delete, get, post, put},
    Router,
};
use axum_login::login_required;
use axum_login::{
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder, AuthUser, AuthnBackend, UserId,
};
use dashmap::{DashMap, DashSet};
use inquire::{min_length, Password, Text};
use password_auth::generate_hash;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::{
    fs::{read, write},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};

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
    levels: DashMap<Key, Level<Validated>>,
    /// Every key and its matching validated orphan (see [`orphanage`]).
    orphans: DashMap<Key, Level<Validated>>,
    banned_ips: DashSet<IpAddr>,
}

impl AppState {
    /// Creates a new, empty Voyager database.
    #[must_use]
    fn new() -> SharedAppState {
        Arc::new(Self {
            levels: DashMap::new(),
            orphans: DashMap::new(),
            banned_ips: DashSet::new(),
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
    pub fn insert(&self, level: Level<Validated>) {
        self.levels.insert(level.key, level);
        self.save();
    }

    /// Inserts an orphan and its key and saves to a file.
    pub fn insert_orphan(&self, level: Level<Validated>) {
        self.orphans.insert(level.key, level);
        self.save();
    }

    /// Checks if the database contains the specified key.
    #[must_use]
    pub fn contains(&self, input: &Key) -> bool {
        self.levels.contains_key(input)
    }

    pub fn ip_is_banned(&self, input: &IpAddr) -> bool {
        self.banned_ips.contains(input)
    }

    /// Moves a level and its key from the orphans list
    /// to the levels list, if found.
    pub fn adopt_orphan(&self, input: &Key) -> Result<()> {
        let (_, level) = self.orphans.remove(input).ok_or(Error::LevelNotFound)?;
        self.insert(level);
        Ok(())
    }

    /// Get a clone of a level from the database, if it exists.
    pub fn get(&self, input: &Key) -> Result<Level<Validated>> {
        self.levels
            .get(input)
            .map_or_else(|| Err(Error::LevelNotFound), |level| Ok(level.clone()))
    }

    /// Deletes a level from the database, if it exists.
    pub fn delete(&self, input: &str) -> Result<StatusCode> {
        let key = input.parse::<Key>()?;
        let deleted = self.levels.remove(&key).is_some();
        self.save();
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(Error::LevelNotFound)
        }
    }

    pub fn ban(&self, input: &str) -> Result<()> {
        let ip = input.parse::<IpAddr>()?;
        self.banned_ips.insert(ip);
        self.save();
        Ok(())
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

    // TODO: this function is a whole mess!
    #[must_use]
    pub fn parsed_levels(&self) -> Vec<Parsed> {
        self.levels
            .clone()
            .into_read_only()
            .values()
            .cloned()
            .filter_map(|level| level.into_parsed().ok())
            .collect::<Vec<Parsed>>()
    }
}

/// Starts the Voyager server on port 3000.
///
/// # Errors
/// Returns an error if the app could not be served.
pub async fn start_voyager() -> Result<()> {
    info!("Voyager is now listening on port 3000.");
    let router = create_router()?;
    serve_app(router).await
}

/// Creates a new [`Router`] for Voyager.
fn create_router() -> Result<Router> {
    let levels = AppState::load();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let backend = Backend::new()?;
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    Ok(Router::new()
        .route("/voyager/webui", get(webui::index::index))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/delete/:key", post(webui::delete::delete))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/ban/:ip", post(webui::ban::ban))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/login", get(webui::login::get))
        .route("/voyager/webui/login", post(webui::login::post))
        .route("/voyager", get(routers::get::get))
        .route("/voyager/:keys", get(routers::get::levels_exist))
        .route("/voyager", post(routers::post::post))
        .route("/voyager/orphanage", post(routers::post::orphanage))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
        .with_state(levels)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(auth_layer))
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

#[derive(Debug, Clone)]
pub struct User {
    id: i64,
    pub username: String,
    password_hash: String,
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Clone, Default)]
pub struct Backend {
    users: std::collections::HashMap<i64, User>,
}

impl Backend {
    fn new() -> Result<Self> {
        println!("please create a user for the webui!");
        let username = Text::new("username:")
            .with_validator(min_length!(2))
            .prompt()?;
        let password = Password::new("password:")
            .with_validator(min_length!(8))
            .prompt()?;
        Ok(Self {
            users: std::collections::HashMap::from([(
                1,
                User {
                    id: 1,
                    username,
                    password_hash: generate_hash(password),
                },
            )]),
        })
    }
}

#[derive(Clone, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub next: Option<String>,
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    async fn authenticate(
        &self,
        Credentials { username, .. }: Self::Credentials,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        Ok(self
            .users
            .values()
            .find(|user| user.username == username)
            .cloned())
    }

    async fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        Ok(self.users.get(user_id).cloned())
    }
}
