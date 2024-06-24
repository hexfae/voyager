//! Contains [`AppState`], related methods, and
//! various Axum server-related functions.
use crate::prelude::*;
use crate::utils::{
    level::{IndexLevel, Validated},
    routers, webui,
};
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
use derive_more::Display;
use inquire::{min_length, Password, Text};
use parking_lot::RwLock;
use password_auth::{generate_hash, verify_password};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
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
use tracing::{debug, error, info, warn};

// for documentation
#[allow(unused_imports)]
use crate::utils::parser;
#[allow(unused_imports)]
use crate::utils::{level::Data, routers::post::orphanage, routers::version::version};
#[allow(unused_imports)]
use base64::prelude::BASE64_STANDARD;

/// List of default allowed songs.
///
/// An empty string (`""`) is allowed and means ambience.
const DEFAULT_ALLOWED_SONGS: [&str; 18] = [
    "", // ambience
    "msc_001",
    "msc_dungeon_wings",
    "msc_beecircle",
    "msc_dungeongroove",
    "msc_013",
    "msc_gorcircle_lo",
    "msc_levcircle",
    "msc_escapewithfriend",
    "msc_cifcircle",
    "msc_006",
    "msc_beesong",
    "msc_themeofcif",
    "msc_monstrail",
    "msc_endless",
    "msc_stg_extraboss",
    "msc_rytmi2",
    "msc_test2",
];

/// The default format version used by Voyager.
///
/// At the time of writing (2024-06-25), this is either `1` or `2`.
///
/// The only difference between these two versions is that version `2` allows Brainfuck symbols.
const DEFAULT_FORMAT_VERSION: u8 = 2;

/// The default latest version of
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// This is used to inform
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// users of new updates through [`version`].
///
/// As of 2024-06-25, this is `0.89`.
const DEFAULT_ENDLESS_VOID_VERSION: &str = "0.89";

/// Voyager's data and configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppState {
    /// Voyager's data (levels, orphans, banned IPs).
    data: VoyagerData,
    /// Voyager's configuration options.
    pub config: RwLock<VoyagerConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize, Display)]
#[display("{} levels, {} orphans, {} banned IPs", levels.len(), orphans.len(), banned_ips.len())]
struct VoyagerData {
    /// Every key and its matching uploaded, validated level.
    levels: DashMap<Key, Level<Validated>>,
    /// Every key and its matching validated orphan (see [`orphanage`]).
    orphans: DashMap<Key, Level<Validated>>,
    /// Every banned IP address. Bans are given out manually in the Web UI.
    banned_ips: DashSet<IpAddr>,
}

#[derive(Debug, Default, Serialize, Deserialize, Display)]
#[display("{} allowed songs, format version {}, Endless Void version {}", allowed_songs.0.len(), format_version, endless_void_version)]
pub struct VoyagerConfig {
    /// All available music choices in Void Stranger.
    ///
    /// See [`DEFAULT_ALLOWED_SONGS`] for the default list.
    #[serde(default)]
    pub allowed_songs: AllowedSongs,
    /// The current highest format version used by
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
    ///
    /// At the time of writing (2024-06-25), this is `2`.
    #[serde(default)]
    pub format_version: FormatVersion,
    /// The version number of the current latest release of
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
    ///
    /// At the time of writing (2024-06-25), this is `0.89`.
    #[serde(default)]
    pub endless_void_version: EndlessVoidVersion,
}

/// The list of allowed songs.
///
/// See [`DEFAULT_ALLOWED_SONGS`] for the list of defaults.
#[derive(Debug, Serialize, Deserialize)]
pub struct AllowedSongs(pub Vec<String>);

/// The latest level format version.
///
/// See [`DEFAULT_FORMAT_VERSION`] for the default.
#[derive(Debug, Serialize, Deserialize, Display)]
pub struct FormatVersion(pub u8);

/// The latest
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// version.
///
/// See [`DEFAULT_ENDLESS_VOID_VERSION`] for the default.
#[derive(Debug, Serialize, Deserialize, Display, Clone)]
pub struct EndlessVoidVersion(pub String);

/// The old, legacy, deprecated, etc. Voyager config.
///
/// The reason why this exists is because I originally thought that the config
/// would be simple enough as to not need to use the newtype pattern. Maybe so,
/// but I've later decided that I do actually want to use it. Since RON doesn't
/// allow you to #[serde(flatten)] newtype structs, this has to exist to convert
/// from the old config to the new one using the newtype pattern.
#[derive(Deserialize)]
struct LegacyVoyagerConfig {
    /// The list of allowed songs.
    ///
    /// See [`DEFAULT_ALLOWED_SONGS`] for the list of defaults.
    allowed_songs: Vec<String>,
    /// The list of allowed characters for tiles and objects.
    ///
    /// This is deprecated. Since Voyager 0.9.0, a new, more sophisticated parser
    /// is used to validate a level's tiles and objects. Therefore, this is no
    /// longer needed. See [`parser`] for the new parser.
    #[allow(dead_code)] // deprecated field
    allowed_characters: String,
    /// The latest level format version.
    ///
    /// See [`DEFAULT_FORMAT_VERSION`] for the default.
    format_version: u8,
    /// The latest version of
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
    ///
    /// See [`DEFAULT_ENDLESS_VOID_VERSION`] for the default.
    endless_void_version: String,
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
    pub fn try_load() -> Result<SharedAppState> {
        debug!("App state is loading...");
        let data = VoyagerData::try_load()?;
        let config = RwLock::new(VoyagerConfig::try_load()?);
        debug!("App state loaded.");

        Ok(Arc::new(Self { data, config }))
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
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    fn backup(&self) {
        let now = OffsetDateTime::now_utc()
            // 2024-05-13
            .date()
            .to_string();
        let path = format!("voyager/backups/{now}.db");
        debug!("Backup is being created at {path}...");

        match bincode::serialize(&self) {
            Err(why) => {
                warn!("Backup could not be created: {why}");
            }
            Ok(bytes) => {
                debug!("Backup created. Backup is saving...");
                let len = bytes.len();
                if let Err(why) = write(path, bytes) {
                    warn!("Backup could not be saved: {why}");
                } else {
                    debug!("Backup saved: {len} bytes.");
                }
            }
        }
    }

    /// Attempts to hot reload the config.
    ///
    /// See [`VoyagerConfig::try_load()`] for details.
    pub fn reload_config(&self) {
        if let Some(config) = VoyagerConfig::try_reload() {
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

    /// Returns the amount of levels in the database.
    pub fn levels_len(&self) -> usize {
        self.data.levels.len()
    }

    /// Returns a comma-separated list of all stored levels in
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)'s
    /// level format.
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
    pub fn index_levels(&self) -> Vec<IndexLevel> {
        self.data
            .levels
            .clone()
            .into_read_only()
            .values()
            .cloned()
            .filter_map(|level| level.into_parsed(&self.config.read()).ok())
            .map(IndexLevel::new)
            .collect()
    }
}

impl VoyagerData {
    /// Attempts to load a Voyager database from `voyager/levels.db`.
    ///
    /// If reading the file fails (likely due to it not yet existing),
    /// it instead creates a new one using `Self::default()`, which
    /// creates an empty database.
    ///
    /// # Errors
    /// - [`Error::Bincode`] if deserializing the file fails.
    fn try_load() -> Result<Self> {
        debug!("Database is opening...");
        read("voyager/levels.db").map_or_else(
            |_| {
                info!("Existing database not found! One will be created.");
                Ok(Self::default())
            },
            |bytes| {
                debug!("Database opened. Database is loading...");
                match bincode::deserialize(&bytes) {
                    Err(why) => {
                        error!("Database could not be loaded! {why}");
                        Err(why.into())
                    }
                    Ok(data) => {
                        info!("Database loaded: {data}.");
                        Ok(data)
                    }
                }
            },
        )
    }

    /// Attempts to save itself to `voyager/levels.db`.
    ///
    /// If an error occurs, it will log a warning and keep running.
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    pub fn save(&self) {
        debug!("Database is serializing...");
        match bincode::serialize(&self) {
            Err(why) => warn!("Database could not be serialized! {why}"),
            Ok(bytes) => {
                let len = bytes.len();
                debug!("Database serialized. Database is saving...");
                match write("voyager/levels.db", bytes) {
                    Ok(()) => debug!("Database saved. {len} bytes."),
                    Err(why) => warn!("Database could not be saved! {why}"),
                };
            }
        };
    }
}

impl VoyagerConfig {
    /// Attempts to save itself to `voyager/config.ron`.
    ///
    /// If an error occurs, it will log a warning and keep running.
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    pub fn save(&self) {
        debug!("Config is serializing...");
        match ron::ser::to_string_pretty(&self, PrettyConfig::default()) {
            Err(why) => warn!("Config could not be serialized! {why}"),
            Ok(string) => {
                debug!("Config serialized. Config is saving...");
                let len = string.len();
                match write("voyager/config.ron", string) {
                    Ok(()) => debug!("Config saved. {len} bytes."),
                    Err(why) => warn!("Config could not be saved! {why}"),
                };
            }
        };
    }

    /// Attempts to load a Voyager config from `voyager/config.ron`.
    ///
    /// If it fails (likely due to it not yet existing), it
    /// instead creates a new one using `Self::default()`,
    /// which will use a set of at-the-time correct defaults.
    ///
    /// # Errors
    /// Returns an error if a Voyager config is found, but
    /// deserializing it fails. Most likely, some data structure
    /// had a breaking change (or the file is corrupted).
    pub fn try_load() -> Result<Self> {
        debug!("Config is opening...");
        read_to_string("voyager/config.ron").map_or_else(
            |_| {
                info!("Existing config not found! One will be created.");
                let config = Self::default();
                config.save();
                Ok(config)
            },
            |string| {
                debug!("Config opened. Config is loading...");
                ron::from_str::<Self>(&string).map_or_else(
                    // TODO: remove
                    #[allow(clippy::cognitive_complexity)]
                    |_| {
                        debug!("Config could not be loaded. Trying legacy...");
                        match ron::from_str::<LegacyVoyagerConfig>(&string) {
                            Err(why) => {
                                error!("Config could not be loaded! {why}");
                                Err(why.into())
                            }
                            Ok(legacy) => {
                                info!("Legacy config loaded. Converting...");
                                let config = Self::from(legacy);
                                info!("Config converted. {config}");
                                config.save();
                                Ok(config)
                            }
                        }
                    },
                    |config| {
                        info!("Config loaded: {config}.");
                        config.save();
                        Ok(config)
                    },
                )
            },
        )
    }

    /// Attempts to load a Voyager config from `voyager/config.ron`.
    ///
    /// This function is used for hot reloading the config
    /// while Voyager is running.
    ///
    /// If it fails (likely due to the configuration being
    /// changed to something invalid), it logs it and keeps
    /// running without switching to the new config.
    pub fn try_reload() -> Option<Self> {
        debug!("Config is opening for hot reload...");
        read_to_string("voyager/config.ron").map_or_else(
            |why| {
                warn!("Config could not be opened for hot reload! {why}");
                None
            },
            |string| {
                debug!("Config opened. Config is loading...");
                match ron::from_str(&string) {
                    Err(why) => {
                        warn!("Config could not be loaded! {why}");
                        None
                    }
                    Ok(config) => {
                        info!("Config reloaded: {config}");
                        Some(config)
                    }
                }
            },
        )
    }
}

impl From<LegacyVoyagerConfig> for VoyagerConfig {
    fn from(input: LegacyVoyagerConfig) -> Self {
        Self {
            allowed_songs: AllowedSongs(input.allowed_songs),
            format_version: FormatVersion(input.format_version),
            endless_void_version: EndlessVoidVersion(input.endless_void_version),
        }
    }
}

impl AllowedSongs {
    /// Returns `true` if the song is in the list of allowed songs.
    pub fn contains(&self, input: impl AsRef<str>) -> bool {
        self.0.iter().any(|s| s == input.as_ref())
    }
}

impl Default for AllowedSongs {
    fn default() -> Self {
        Self(DEFAULT_ALLOWED_SONGS.map(ToString::to_string).to_vec())
    }
}

impl Default for FormatVersion {
    fn default() -> Self {
        Self(DEFAULT_FORMAT_VERSION)
    }
}

impl Default for EndlessVoidVersion {
    fn default() -> Self {
        Self(DEFAULT_ENDLESS_VOID_VERSION.into())
    }
}

/// Starts the Voyager server on port 3000.
///
/// # Errors
/// Returns an error if the app could not be served.
pub async fn start_voyager(app_state: SharedAppState, backend: Backend) -> Result<()> {
    tokio::spawn(backup_state_daily(app_state.clone()));
    let router = create_router(app_state, backend);
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
fn create_router(app_state: SharedAppState, backend: Backend) -> Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    Router::new()
        .route("/voyager/webui", get(webui::index::index))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/levels", get(webui::index::levels))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/delete/:key", post(webui::index::delete))
        .route_layer(login_required!(Backend, login_url = "/voyager/webui/login"))
        .route("/voyager/webui/ban/:ip", post(webui::index::ban))
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
        .layer(auth_layer)
}

/// Serves the Voyager app on port 3000.
async fn serve_app(app: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Voyager started. Listening on port 3000...");
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
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    pub fn try_load() -> Result<Self> {
        debug!("Web UI user is opening...");
        let input = read("voyager/webui.db");
        input.map_or_else(
            |_| {
                info!("Existing Web UI user not found! Please create one...");
                Self::new()
            },
            |bytes| {
                debug!("Web UI user opened. Web UI user loading...");
                let backend = Self::from(&bytes);
                match backend {
                    Err(why) => {
                        error!("Web UI user could not be loaded! {why}");
                        Err(why)
                    }
                    Ok(backend) => {
                        // why does Values have a last() method but not a first() method?
                        let user = backend.users.values().last().cloned();
                        user.map_or_else(
                            || {
                                error!("Web UI user could not be found!");
                                Err(Error::WebUI)
                            },
                            |user| {
                                info!("Web UI user loaded: {}.", user.username);
                                Ok(backend)
                            },
                        )
                    }
                }
            },
        )
    }

    /// Attempts to save itself to `voyager/webui.db`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    fn save(&self) {
        debug!("Web UI user is serializing...");
        match bincode::serialize(&self) {
            Ok(bytes) => {
                debug!("Web UI user serialized. Web UI is saving...");
                if let Err(why) = write("voyager/webui.db", bytes) {
                    warn!("Web UI user could not be saved: {why}");
                } else {
                    debug!("Web UI user saved.");
                };
            }
            Err(why) => warn!("Web UI could not be serialized: {why}"),
        }
    }

    /// Attempts to deserialize a Web UI user from bytes.
    ///
    /// # Errors
    /// This function will return an error if deserializing
    /// it fails. Most likely, some data structure had a
    /// breaking change (or the file is corrupted).
    fn from(webui: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(webui)?)
    }

    /// Asks for a username and password on the CLI.
    ///
    /// The name must be at least 2 characters long.
    /// The password must be at least 8 characters long.
    fn new() -> Result<Self> {
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
