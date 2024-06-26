//! Contains [`AppState`], related methods, and
//! various Axum server-related functions.

use crate::prelude::*;
use crate::utils::{
    level::{Author, IndexLevel, Name, Validated},
    routers, webui,
};
use axum::{
    http::StatusCode,
    routing::{any, delete, get, post, put},
    Router,
};
use axum_login::login_required;
use axum_login::{
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use base64::Engine;
use dashmap::{DashMap, DashSet};
use derive_more::Display;
use itertools::Itertools;
use parking_lot::RwLock;
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
/// The only difference between these two versions is that version `2`'s Add statues support
/// [Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck).
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
    pub data: VoyagerData,
    /// Voyager's configuration options.
    pub config: RwLock<VoyagerConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize, Display)]
#[display("{} levels, {} orphans, {} banned IPs", levels.len(), orphans.len(), banned_ips.len())]
pub struct VoyagerData {
    /// Every key and its matching uploaded, validated level.
    pub levels: DashMap<Key, Level<Validated>>,
    /// Every key and its matching validated orphan (see [`orphanage`]).
    pub orphans: DashMap<Key, Level<Validated>>,
    /// Every banned IP address. Bans are given out manually in the Web UI.
    pub banned_ips: DashSet<IpAddr>,
}

#[derive(Debug, Default, Serialize, Deserialize, Display)]
#[display("{} allowed songs, format version {}, Endless Void version {}", allowed_songs.0.len(), latest_format_version, latest_endless_void_version)]
pub struct VoyagerConfig {
    /// See [`AllowedSongs`].
    #[serde(default)]
    pub allowed_songs: AllowedSongs,
    /// See [`LatestFormatVersion`].
    #[serde(default)]
    #[serde(alias = "format_version")]
    pub latest_format_version: LatestFormatVersion,
    /// See [`LatestEndlessVoidVersion`].
    #[serde(default)]
    #[serde(alias = "endless_void_version")]
    pub latest_endless_void_version: LatestEndlessVoidVersion,
    /// See [`DiscordWebhookUrl`].
    #[serde(default)]
    pub discord_webhook_url: DiscordWebhookUrl,
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
pub struct LatestFormatVersion(pub u8);

/// The latest
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// version.
///
/// See [`DEFAULT_ENDLESS_VOID_VERSION`] for the default.
#[derive(Debug, Serialize, Deserialize, Display, Clone)]
pub struct LatestEndlessVoidVersion(pub String);

/// The Discord webhook URL used for level upload messages.
///
/// Voyager can optionally send a Discord message using
/// the provided webhook URL when a level is uploaded.
///
/// The default is an empty string (`""`), which will skip
/// trying to send the message altogether. A webhook URL must
/// be set through the config.
#[derive(Debug, Display, Default, Serialize, Deserialize)]
pub struct DiscordWebhookUrl(String);

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
    /// a default one.
    ///
    /// # Errors
    /// Returns an error if a Voyager database is
    /// found, but deserializing it fails. Most
    /// likely, some data structure had a breaking
    /// change (or the file is corrupted).
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

    /// Adopts an orphan with the input key.
    ///
    /// Moves a level and its key from the orphans list
    /// to the levels list, if found.
    pub fn adopt_orphan(&self, input: &Key) -> Result<()> {
        let (_, level) = self
            .data
            .orphans
            .remove(input)
            .ok_or(Error::LevelNotFound)?;
        self.send_discord_message(level.clone());
        self.insert(level);
        Ok(())
    }

    /// Sends a Discord message about the input level.
    ///
    /// If set in the config, attempt to send a Discord message using
    /// the configured webhook URL with information about the level.
    ///
    /// This is used to optionally notify a Discord channel when a level
    /// is uploaded to Voyager.
    fn send_discord_message(&self, level: Level<Validated>) {
        let webhook_url = self.config.read().discord_webhook_url.to_string();
        if webhook_url.is_empty() {
            return;
        }
        let Ok(parsed) = level.into_parsed(&self.config.read()) else {
            warn!("could not parse level for some reason?");
            return;
        };
        let level = IndexLevel::new(parsed);
        let embed = ureq::json!({
            "embeds": [{
                "title": level.name,
                "description": level.description,
                "author": {
                    "name": level.author
                }
            }]
        })
        .to_string();
        tokio::task::spawn_blocking(move || {
            let message = ureq::post(&webhook_url)
                .set("Content-Type", "application/json")
                .send_string(&embed);
            if let Err(why) = message {
                warn!("could not send discord webhook message on level upload! {why}");
            }
        });
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

    /// Checks a name and author against the database for collisions.
    ///
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
    /// levels must be uniquely identifiable, which may be done through checking
    /// a level's name and author. Two levels can not have the same name and author.
    ///
    /// # Errors
    /// Returns an error if the database already contains a level with the same name
    /// and author as the input.
    // due to the #[cfg(debug_assertions)], the Err variant is
    // unreachable in dev/clippy, which creates a warning. ignore it
    #[allow(unreachable_code)]
    pub fn check_for_name_and_author_collisions(
        &self,
        name: impl AsRef<str>,
        author: impl AsRef<str>,
    ) -> Result<()> {
        let names_and_authors = self
            .data
            .levels
            .clone()
            .into_iter()
            .map(|(_key, level)| level)
            .filter_map(|level| {
                let string = level.data.to_string();
                let (_version, name, _description, _music, author, _other) =
                    string.splitn(6, '|').collect_tuple()?;
                let name = String::from_utf8(BASE64_STANDARD.decode(name).ok()?).ok()?;
                let author = String::from_utf8(BASE64_STANDARD.decode(author).ok()?).ok()?;
                Some((Name(name), Author(author)))
            })
            .collect_vec();
        if names_and_authors
            .into_iter()
            .any(|(n, a)| n.0 == name.as_ref() && a.0 == author.as_ref())
        {
            info!("POST failed! level/name collision");
            #[cfg(debug_assertions)]
            {
                info!("Running in dev mode; name/author collision will be ignored.");
                return Ok(());
            }
            return Err(Error::LevelNameCollision);
        }
        Ok(())
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
            latest_format_version: LatestFormatVersion(input.format_version),
            latest_endless_void_version: LatestEndlessVoidVersion(input.endless_void_version),
            ..Default::default()
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

impl Default for LatestFormatVersion {
    fn default() -> Self {
        Self(DEFAULT_FORMAT_VERSION)
    }
}

impl Default for LatestEndlessVoidVersion {
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
