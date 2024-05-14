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
use parking_lot::RwLock;
use password_auth::{generate_hash, verify_password};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string};
use std::net::IpAddr;
use std::{
    fs::{read, write},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use time::OffsetDateTime;
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};

// for documentation
#[allow(unused_imports)]
use crate::utils::{level::Data, routers::post::orphanage};

/// Thread-safe app state, used across Voyager.
pub type SharedAppState = Arc<AppState>;

/// Voyager's data and configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    /// Voyager's data (levels, orphans, banned IPs).
    data: VoyagerData,
    /// Voyager's configuration options.
    pub config: RwLock<VoyagerConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VoyagerData {
    /// Every key and its matching uploaded, validated level.
    levels: DashMap<Key, Level<Validated>>,
    /// Every key and its matching validated orphan (see [`orphanage`]).
    orphans: DashMap<Key, Level<Validated>>,
    /// Every banned IP address. Bans are given out manually in the Web UI.
    banned_ips: DashSet<IpAddr>,
}

impl VoyagerData {
    /// Attempts to save itself to `voyager/levels.db`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    pub fn save(&self) {
        match bincode::serialize(&self) {
            Ok(bytes) => {
                if let Err(why) = write("voyager/levels.db", bytes) {
                    warn!("database could not be saved: {why}");
                };
            }
            Err(why) => warn!("database could not be serialized: {why}"),
        };
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoyagerConfig {
    /// All available music choices in Void Stranger.
    ///
    /// Note: An empty string (`""`) is allowed and means ambience.
    pub allowed_songs: Vec<String>,
    /// The current highest format version used by Endless Void.
    ///
    /// At the time of writing (2024-05-14), this is `2`.
    pub format_version: u8,
    /// The version number of the current latest release of Endless Void.
    ///
    /// At the time of writing (2024-05-14), this is `0.875`.
    pub endless_void_version: String,
}

impl VoyagerConfig {
    /// Attempts to save itself to `voyager/config.ron`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    pub fn save(&self) {
        match ron::ser::to_string_pretty(&self, PrettyConfig::default()) {
            Ok(bytes) => {
                if let Err(why) = write("voyager/config.ron", bytes) {
                    warn!("config could not be saved: {why}");
                };
            }
            Err(why) => warn!("config could not be serialized: {why}"),
        };
    }

    /// Attempts to load a Voyager config from `voyager/config.ron`.
    ///
    /// If it fails (likely due to it not yet existing), it
    /// instead creates a new one using `Self::default()`,
    /// which will use a set of at-the-time correct defaults.
    ///
    /// # Panics
    /// Panics if a Voyager config is found, but deserializing
    /// it fails. Most likely, some data structure had a
    /// breaking change (or the file is corrupted).
    pub fn load() -> RwLock<Self> {
        read_to_string("voyager/config.ron").map_or_else(
            |_| {
                info!("Existing config not found! One will be created...");
                let config = Self::default();
                config.save();
                RwLock::new(config)
            },
            |string| {
                info!("Existing config found.");
                ron::from_str(&string).expect("valid config file")
            },
        )
    }

    /// Attempts to load a Voyager config from `voyager/config.ron`.
    ///
    /// This function is used for hot-reloading the config
    /// while Voyager is running.
    ///
    /// If it fails (likely due to the configuration being
    /// changed to something invalid), it logs it and keeps
    /// running without switching to the new config.
    pub fn try_load() -> Option<Self> {
        read_to_string("voyager/config.ron").map_or_else(
            |why| {
                warn!("could not read config to hot-reload: {why}");
                None
            },
            |string| match ron::from_str(&string) {
                Err(why) => {
                    warn!("could not hot-reload config: {why}");
                    None
                }
                Ok(config) => {
                    info!("Hot-reloaded config.");
                    Some(config)
                }
            },
        )
    }
}

impl Default for VoyagerConfig {
    fn default() -> Self {
        Self {
            allowed_songs: vec![
                String::new(), // "", ambience
                "msc_001".into(),
                "msc_dungeon_wings".into(),
                "msc_beecircle".into(),
                "msc_dungeongroove".into(),
                "msc_013".into(),
                "msc_gorcircle_lo".into(),
                "msc_levcircle".into(),
                "msc_escapewithfriend".into(),
                "msc_cifcircle".into(),
                "msc_006".into(),
                "msc_beesong".into(),
                "msc_themeofcif".into(),
                "msc_monstrail".into(),
                "msc_endless".into(),
                "msc_stg_extraboss".into(),
                "msc_rytmi2".into(),
                "msc_test2".into(),
            ],
            format_version: 2,
            endless_void_version: "0.875".into(),
        }
    }
}

impl AppState {
    /// Attempts to load a Voyager database from
    /// `voyager/levels.db`. If it fails (likely due
    /// to it not yet existing), it instead creates
    /// a new one using `Self::new()`.
    ///
    /// # Panics
    /// Panics if a Voyager database is found, but
    /// deserializing it fails. Most likely, some
    /// data structure had a breaking change (or
    /// the file is corrupted).
    #[must_use]
    pub fn load() -> SharedAppState {
        let data = read("voyager/levels.db").map_or_else(
            |_| {
                info!("Existing database not found! One will be created...");
                VoyagerData::default()
            },
            |bytes| {
                info!("Existing database found.");
                bincode::deserialize(&bytes).expect("valid database file")
            },
        );

        let config = VoyagerConfig::load();

        Arc::new(Self { data, config })
    }

    /// Attempts to save itself to `voyager/levels.db`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    fn save(&self) {
        self.data.save();
    }

    /// Performs a backup to `voyager/backups/yyyy-mm-dd.db`.
    ///
    /// Used for daily backups.
    fn backup(&self) {
        let now = OffsetDateTime::now_utc()
            // 2024-05-13
            .date()
            .to_string();
        let path = format!("voyager/backups/{now}.db");

        match bincode::serialize(&self) {
            Ok(bytes) => {
                let len = bytes.len();
                if let Err(why) = write(path, bytes) {
                    warn!("database could not be saved: {why}");
                } else {
                    info!("Backup saved: {len} bytes");
                }
            }
            Err(why) => {
                warn!("database could not be serialized: {why}");
            }
        }
    }

    /// Attempts to hot-reload the config.
    ///
    /// See [`VoyagerConfig::try_load()`] for details.
    pub fn reload_config(&self) {
        if let Some(config) = VoyagerConfig::try_load() {
            *self.config.write() = config;
        }
    }

    /// Inserts a level and its key and saves to a file.
    pub fn insert(&self, level: Level<Validated>) {
        self.data.levels.insert(level.key, level);
        self.save();
    }

    /// Inserts an orphan and its key and saves to a file.
    pub fn insert_orphan(&self, level: Level<Validated>) {
        self.data.orphans.insert(level.key, level);
        self.save();
    }

    /// Checks if the database contains the specified key.
    #[must_use]
    pub fn contains(&self, input: &Key) -> bool {
        self.data.levels.contains_key(input)
    }

    /// Checks if the given IP address is banned.
    pub fn ip_is_banned(&self, input: &IpAddr) -> bool {
        self.data.banned_ips.contains(input)
    }

    /// Moves a level and its key from the orphans list
    /// to the levels list, if found.
    pub fn adopt_orphan(&self, input: &Key) -> Result<()> {
        let (_, level) = self
            .data
            .orphans
            .remove(input)
            .ok_or(Error::LevelNotFound)?;
        self.insert(level);
        Ok(())
    }

    /// Get a clone of a level from the database, if it exists.
    pub fn get(&self, input: &Key) -> Result<Level<Validated>> {
        self.data
            .levels
            .get(input)
            .map_or_else(|| Err(Error::LevelNotFound), |level| Ok(level.clone()))
    }

    /// Deletes a level from the database, if it exists.
    pub fn delete(&self, input: &Key) -> Result<StatusCode> {
        let deleted = self.data.levels.remove(input).is_some();
        self.save();
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(Error::LevelNotFound)
        }
    }

    /// Bans the specified IP address from Voyager.
    ///
    /// Specifically, it will add the IP address to
    /// the ban list, as well as delete all levels
    /// uploaded by that IP address.
    pub fn ban(&self, input: &str) -> Result<()> {
        let ip = input.parse::<IpAddr>()?;
        self.data.banned_ips.insert(ip);
        // clone because dashmap will deadlock otherwise
        let levels = self.data.levels.clone();
        for level in &levels {
            if level.uploader == ip {
                self.delete(&level.key)?;
            }
        }
        self.save();
        Ok(())
    }

    /// Returns a comma-separated list of all stored levels
    /// in Endless Void's level format.
    ///
    /// See [`Data`] for details on level format.
    #[must_use]
    pub fn levels(&self) -> String {
        self.data
            .levels
            .clone()
            .into_read_only()
            .values()
            .map(|level| level.data.to_string())
            .collect::<Vec<String>>()
            .join(",")
    }

    // TODO: this function is a whole mess!
    #[must_use]
    /// Parses and returns all stored levels.
    pub fn parsed_levels(&self) -> Vec<Parsed> {
        self.data
            .levels
            .clone()
            .into_read_only()
            .values()
            .cloned()
            .filter_map(|level| level.into_parsed(&self.config.read()).ok())
            .collect::<Vec<Parsed>>()
    }
}

/// Starts the Voyager server on port 3000.
///
/// # Errors
/// Returns an error if the app could not be served.
pub async fn start_voyager(app_state: SharedAppState) -> Result<()> {
    info!("Voyager is now listening on port 3000.");
    let _ = create_dir_all("voyager/backups");
    tokio::spawn(backup_state_daily(app_state.clone()));
    let router = create_router(app_state)?;
    serve_app(router).await
}

/// Performs daily backups at `voyager/backups/yyyy-mm-dd`.
async fn backup_state_daily(app_state: SharedAppState) {
    let one_day = Duration::from_secs(60 * 60 * 24);
    let mut interval = tokio::time::interval(one_day);

    loop {
        interval.tick().await;
        app_state.backup();
    }
}

/// Creates a new [`Router`] for Voyager.
fn create_router(app_state: SharedAppState) -> Result<Router> {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let backend = Backend::load()?;
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
        .route("/voyager/version", get(routers::version::version))
        .route("/voyager", post(routers::post::post))
        .route("/voyager/orphanage", post(routers::post::orphanage))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
        .with_state(app_state)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A Web UI user. Used for administrative tasks.
pub struct User {
    /// User's id. Always 1.
    ///
    /// This is because the current implementation
    /// only allows for one user (an admin).
    id: i64,
    /// User's username.
    pub username: String,
    /// User's password hash.
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

#[derive(Clone, Default, Serialize, Deserialize)]
/// Web UI backend.
pub struct Backend {
    /// All Web UI users.
    ///
    /// Currently, there can only be one (an admin).
    users: std::collections::HashMap<i64, User>,
}

impl Backend {
    /// Attempts to load a Web UI user from
    /// `voyager/webui.db`. If it fails (likely due
    /// to it not yet existing), it instead creates
    /// a new one using `Self::new()`, which will
    /// ask for a username and password.
    ///
    /// # Panics
    /// Panics if a Voyager database is found, but
    /// deserializing it fails. Most likely, some
    /// data structure had a breaking change (or
    /// the file is corrupted).
    fn load() -> Result<Self> {
        let input = read("voyager/webui.db");
        input.map_or_else(
            |_| {
                info!("Existing Web UI user not found! One will be created...");
                Self::new()
            },
            |bytes| {
                info!("Existing Web UI user found.");
                Ok(Self::from(&bytes))
            },
        )
    }

    /// Attempts to save itself to `voyager/webui.db`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    fn save(&self) {
        match bincode::serialize(&self) {
            Ok(bytes) => {
                if let Err(why) = write("voyager/webui.db", bytes) {
                    warn!("webui could not be saved: {why}");
                };
            }
            Err(why) => warn!("webui could not be serialized: {why}"),
        }
    }

    /// Attempts to deserialize a Web UI user from bytes.
    ///
    /// # Panics
    /// This function will return an error if deserializing
    /// it fails. Most likely, some data structure had a
    /// breaking change (or the file is corrupted).
    fn from(webui: &[u8]) -> Self {
        bincode::deserialize(webui).expect("valid web ui user")
    }

    /// Asks for a username and password on the CLI.
    ///
    /// The name must be at least 2 characters long.
    /// The password must be at least 8 characters long.
    fn new() -> Result<Self> {
        println!("please create a user for the webui!");
        let username = Text::new("username:")
            .with_validator(min_length!(2))
            .prompt()?;
        let password = Password::new("password:")
            .with_validator(min_length!(8))
            .prompt()?;
        let login = Self {
            users: std::collections::HashMap::from([(
                1,
                User {
                    id: 1,
                    username,
                    password_hash: generate_hash(password),
                },
            )]),
        };
        login.save();
        Ok(login)
    }
}

#[derive(Clone, Deserialize)]
/// A user's credentials, used for authentication.
pub struct Credentials {
    /// User's username.
    pub username: String,
    /// User's password.
    ///
    /// Note: This is never stored nor logged. This
    /// is immediately hashed and then dropped.
    pub password: String,
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        Credentials {
            username, password, ..
        }: Self::Credentials,
    ) -> Result<Option<Self::User>> {
        let user = self
            .users
            .values()
            .find(|user| user.username == username)
            .cloned();

        tokio::task::spawn_blocking(|| {
            Ok(user.filter(|user| verify_password(password, &user.password_hash).is_ok()))
        })
        .await?
    }

    async fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        Ok(self.users.get(user_id).cloned())
    }
}
