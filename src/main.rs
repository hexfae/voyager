//! Voyager is the server back-end for
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void),
//! which is a level editor for
//! [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/),
//! a "2D sokoban-style puzzle game where every step counts."
//!
//! It supports uploading levels, editing levels, deleting levels, and
//! downloading all uploaded levels. Authentication is managed through
//! a per-level key-based system.

mod error;
mod prelude;
mod utils;

#[tokio::main]
async fn main() -> prelude::Result<()> {
    // file logger only periodically saves the logs to file.
    // it will also saves the logs to a file when the guard
    // is dropped (at the end of this scope)
    let _guard = start_logging();

    tracing::info!("Voyager is launching.");
    let app_state = utils::server::AppState::load()?;

    // all of this ugliness has to go in main because, if put in a
    // function, the watcher will get dropped too early (at the end
    // of the function) and not actually watch the config (i think)
    let cloned_app_state = app_state.clone();
    let mut debouncer = new_debouncer(
        Duration::from_secs_f64(0.1),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                for event in events {
                    if event.kind == DebouncedEventKind::Any
                        && event.path.ends_with("voyager/config.ron")
                    {
                        cloned_app_state.reload_config();
                    }
                }
            }
            Err(e) => tracing::warn!("watch error: {e:?}"),
        },
    )?;

    debouncer.watcher().watch(
        std::path::Path::new("voyager"),
        notify_debouncer_mini::notify::RecursiveMode::NonRecursive,
    )?;

    utils::server::start_voyager(app_state).await
}

use std::time::Duration;

use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

/// Starts logging.
///
/// This will log at a level of INFO to the terminal, and log at a level of DEBUG to a file.
///
/// The file logs are saved on a per-day basis to `voyager/logs/voyager.log.yyy-mm-dd`.
fn start_logging() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("voyager/logs", "voyager.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let stdout_log = tracing_subscriber::fmt::layer().pretty();
    let file_log = fmt::layer().with_writer(non_blocking);
    tracing_subscriber::registry()
        .with(
            stdout_log
                .with_filter(LevelFilter::INFO)
                .and_then(file_log)
                .with_filter(LevelFilter::DEBUG),
        )
        .init();
    guard
}
