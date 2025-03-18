use std::{fs::create_dir, net::SocketAddr};

use crate::{
    incantations::{adopt, amend, beacon, expunge, inscribe, probe, unveil},
    nexus::Nexus,
};
use axum::{
    Router,
    routing::{get, post},
};
use miette::{Diagnostic, Result};
use owo_colors::OwoColorize;
use snafu::{ResultExt, Snafu};
use tokio::net::TcpListener;
use tracing::{error, level_filters::LevelFilter, warn};
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

pub async fn serve_voyager() -> miette::Result<()> {
    let nexus = Nexus::try_load()?;
    nexus.try_save()?;
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
