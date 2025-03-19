use std::{fs::create_dir, net::SocketAddr, sync::Arc, time::Duration};

use crate::{
    incantations::{adopt, amend, beacon, expunge, inscribe, probe, unveil},
    nexus::{Atlas, Manifest, Nexus},
};
use axum::{
    Router,
    routing::{get, post},
};
use miette::{Diagnostic, IntoDiagnostic, Result};
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, NoCache, new_debouncer, notify::INotifyWatcher,
};
use owo_colors::OwoColorize;
use parking_lot::RwLock;
use snafu::{ResultExt, Snafu};
use tokio::net::TcpListener;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    Layer, Registry, fmt,
    layer::SubscriberExt,
    reload::{Handle, Layer as ReloadLayer},
    util::SubscriberInitExt,
};

const ADDRESS: &str = "0.0.0.0:3000";

/// Creates all the directories needed for Void Voyager.
///
/// # Errors
///
/// Returns an error if any of the directories could not be created.
pub fn create_void_voyager_directories() -> Result<()> {
    try_create_directory("voyager")?;
    try_create_directory("voyager/backups")?;
    try_create_directory("voyager/logs")?;
    Ok(())
}

/// Creates the specified directory if it does not exist.
///
/// # Errors
///
/// It returns an error if the directory could not be created.
fn try_create_directory(path: impl AsRef<str> + Into<String> + Copy) -> Result<()> {
    if let Err(why) = create_dir(path.as_ref()).context(DirectorySnafu { path: path.into() }) {
        if why.source.kind() != std::io::ErrorKind::AlreadyExists {
            error!("while creating directory!");
            return Err(why.into());
        }
    }
    Ok(())
}

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(display("Failed to create directory {}", path.cyan()))]
#[diagnostic(
    code(void_voyager::main::create_void_voyager_directories),
    help("Is the directory writable?")
)]
struct DirectoryError {
    path: String,
    source: std::io::Error,
}

fn event_handler(res: DebounceEventResult, manifest: Arc<RwLock<Manifest>>) {
    match res {
        Err(errors) => {
            for why in errors {
                warn!("while watching config file! {why}");
            }
        }
        Ok(events) => {
            for event in events {
                // certain text editors (e.g. helix) will "modify" a file by creating a temporary
                // file, deleting the original, and then moving (?) the new file over where the
                // original was. i'm sure some other text editors just modify the old file in place
                if (event.kind.is_modify() || event.kind.is_create())
                    && event.paths.iter().any(|path| path.ends_with("config.ron"))
                {
                    if let Err(why) = manifest.write().reload().into_diagnostic() {
                        warn!("Error reloading config!");
                        println!("{why:?}");
                    } else {
                        info!("Reloaded config!");
                    }
                }
            }
        }
    }
}

#[derive(Debug, Snafu, Diagnostic)]
pub enum ConfigWatchError {
    #[snafu(display("Failed to create config debouncer"))]
    #[diagnostic(
        code(void_voyager::main::watch_config),
        help("You're on your own for this one.")
    )]
    DebounceError {
        source: notify_debouncer_full::notify::Error,
    },
    #[snafu(display("Failed to watch config file"))]
    #[diagnostic(
        code(void_voyager::main::watch_config),
        help("You're on your own for this one.")
    )]
    WatchError {
        source: notify_debouncer_full::notify::Error,
    },
}

pub trait WatchVoyagerConfig {
    fn watch_voyager_config(&mut self) -> Result<(), ConfigWatchError>;
}

impl WatchVoyagerConfig for Debouncer<INotifyWatcher, NoCache> {
    fn watch_voyager_config(&mut self) -> Result<(), ConfigWatchError> {
        self.watch(
            "voyager",
            notify_debouncer_full::notify::RecursiveMode::NonRecursive,
        )
        .context(WatchSnafu)
    }
}

pub fn watch_config(
    manifest: Arc<RwLock<Manifest>>,
) -> Result<Debouncer<INotifyWatcher, NoCache>, ConfigWatchError> {
    new_debouncer(Duration::from_secs(1), None, move |res| {
        event_handler(res, manifest.clone())
    })
    .context(DebounceSnafu)
}

pub async fn backup_levels_daily(atlas: Arc<Atlas>) {
    let one_day = Duration::from_secs(60 * 60 * 24);
    let mut interval = tokio::time::interval(one_day);
    loop {
        interval.tick().await;
        atlas.backup();
    }
}

pub async fn serve_voyager(nexus: Nexus) -> miette::Result<()> {
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
        .context(BindSnafu { address: ADDRESS })?;
    axum::serve(listener, app).await.context(ServeSnafu)?;
    Ok(())
}

#[derive(Debug, Snafu, Diagnostic)]
enum AxumError {
    #[snafu(display("Failed to bind to address {}", address.cyan()))]
    #[diagnostic(
        code(void_voyager::main::serve_voyager),
        help("Is the port available?")
    )]
    Bind {
        address: String,
        source: std::io::Error,
    },
    #[snafu(display("Failed to serve Voyager app"))]
    #[diagnostic(
        code(void_voyager::main::serve_voyager),
        help("You're on your own for this one.")
    )]
    Serve { source: std::io::Error },
}

/// Type alias for the handle that is used to reload the logging configuration.
#[allow(clippy::type_complexity)]
pub type ReloadHandle = Handle<Vec<Box<dyn Layer<Registry> + Send + Sync>>, Registry>;

/// Starts logging to stdout.
///
/// It returns a [`ReloadHandle`], which is used to reload the logging configuration.
pub fn stdout_log() -> ReloadHandle {
    let stdout_log = fmt::layer()
        .pretty()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_filter(LevelFilter::INFO)
        .boxed();
    let layers = vec![stdout_log];
    let (tracing_layers, reload_handle) = ReloadLayer::new(layers);
    tracing_subscriber::registry().with(tracing_layers).init();
    reload_handle
}

/// Starts logging to a file.
///
/// It takes in a [`ReloadHandle`], which is used to reload the logging configuration.
/// It returns a [`WorkerGuard`], which is used to flush the logs when the guard is dropped.
///
/// The file logs are saved on a per-day basis to `voyager/logs/voyager.log.yyy-mm-dd`.
pub fn file_log(reload_handle: &ReloadHandle) -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("voyager/logs", "voyager.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let file_log = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .boxed();
    let begin_logging_to_file = reload_handle.modify(|layer| {
        layer.push(file_log);
    });
    if let Err(why) = begin_logging_to_file {
        warn!("Could not begin logging to file! {why}");
    };
    guard
}
