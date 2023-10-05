use std::env;
use tracing_subscriber::filter::LevelFilter;

/// Starts the logging system.
///
/// The logging level is read from the `LOG_LEVEL` environment variable.
/// If this variable is not set, or is set incorretly, the default level is `DEBUG`.
pub fn start_logging() {
    tracing::debug!("starting logging");
    let level: LevelFilter = env::var("LOG_LEVEL")
        .unwrap_or(String::from("DEBUG"))
        .parse()
        .unwrap_or(LevelFilter::DEBUG);

    tracing_subscriber::fmt().with_max_level(level).init();
}
