//! Voyager is the server back-end for
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void),
//! which is a level editor for
//! [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/),
//! a "2D sokoban-style puzzle game where every step counts."
//!
//! It supports uploading levels, editing levels, deleting levels, and
//! downloading all uploaded levels. Authentication is managed through
//! a per-level key-based system.

use crate::{prelude::*, utils::server::start_voyager};
use notify_debouncer_mini::{
    new_debouncer, notify::RecursiveMode, DebounceEventResult, DebouncedEventKind,
};
use std::fs::rename;
use std::{fs::create_dir, io, path::Path, time::Duration};
use tracing::{debug, error, info, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::reload::Layer as ReloadLayer;
use tracing_subscriber::Registry;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use utils::server::AppState;

mod error;
mod prelude;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let handle = stdout_log();
    create_voyager_directories()?;
    // file logger only periodically saves the logs to file.
    // it will also saves the logs to a file when the guard
    // is dropped (at the end of this scope)
    let _guard = file_log(&handle);

    info!("Voyager is loading...");
    let app_state = AppState::try_load()?;
    // save immediately in case a new config option has
    // been added so that #[serde(default)] can create it
    app_state.config.read().save();
    let backend = Backend::try_load()?;

    // all of this ugliness has to go in main because, if put in a
    // function, the watcher will get dropped too early (at the end
    // of the function) and not actually watch the config (i think)
    let cloned_app_state = app_state.clone();
    let mut debouncer = new_debouncer(
        Duration::from_secs_f64(0.1),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                for event in events {
                    if event.kind == DebouncedEventKind::Any && event.path.ends_with("config.ron") {
                        debug!("Config is reloading...");
                        cloned_app_state.reload_config();
                    }
                }
            }
            Err(e) => {
                warn!("watch error: {e:?}");
            }
        },
    )?;

    debug!("Config is being watched for changes.");
    debouncer
        .watcher()
        .watch(Path::new("voyager"), RecursiveMode::NonRecursive)?;

    info!("Voyager loaded. Voyager is starting...");
    start_voyager(app_state, backend).await
}

/// Creates all the directories needed for Voyager and checks if `voyager`
/// is a file.
///
/// # Errors
/// Returns an error if any of the directories could not be created or if
/// `voyager` could not be renamed (if present).
fn create_voyager_directories() -> Result<()> {
    check_if_voyager_is_file()?;
    try_create_directory("voyager")?;
    try_create_directory("voyager/backups")?;
    try_create_directory("voyager/logs")?;
    Ok(())
}

/// Checks if `voyager` is a file. If it is, it renames it to `voyagerexe`.
///
/// This is a workaround. Early in the development process, the decision was
/// made to make `voyager` the name for Voyager's directory. However, it is
/// not possible to have both a file named `voyager` and a directory named
/// `voyager`, which would cause Voyager to panic on startup. This went
/// unnoticed at first due to Voyager always being ran through `cargo run`
/// during development, which meant that the `voyager` executable would
/// not be in the same directory as the created `voyager` directory.
/// Additionally, the [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// developer had coincidentally been renaming `voyager` to `voyagerexe`.
/// When this issue was noticed, it was too late to change the name of the
/// created directory (this would be a breaking change). Therefore, the
/// binaries are now distributed as `voyager-amd64` and `voyager-aarch64`,
/// and the `voyager` executable is renamed to `voyagerexe` if found in
/// the current directory.
///
/// # Errors
/// It returns an error if the executable could not be renamed.
pub fn check_if_voyager_is_file() -> Result<()> {
    if Path::new("voyager").is_file() {
        warn!("could not create voyager directory!");
        info!("renaming the executable to voyagerexe...");
        if let Err(why) = rename("voyager", "voyagerexe") {
            error!("could not rename executable! {why}");
            return Err(Error::Directory);
        };
    }
    Ok(())
}

/// Creates the specified directory if it does not exist.
///
/// # Errors
/// It returns an error if the directory could not be created.
fn try_create_directory(path: &str) -> Result<()> {
    if let Err(why) = create_dir(path) {
        if why.kind() != io::ErrorKind::AlreadyExists {
            error!("could not create {path} directory! {why}");
            return Err(why.into());
        }
    }
    Ok(())
}

/// Type alias for the handle that is used to reload the logging configuration.
#[allow(clippy::type_complexity)]
pub type ReloadHandle = Handle<Vec<Box<dyn Layer<Registry> + Send + Sync>>, Registry>;

/// Starts logging to stdout.
///
/// It returns a [`ReloadHandle`], which is used to reload the logging configuration.
fn stdout_log() -> ReloadHandle {
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
fn file_log(reload_handle: &ReloadHandle) -> WorkerGuard {
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
