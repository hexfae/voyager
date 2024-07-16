//! Void Voyager's comprehensive error type.

// for documentation
#[allow(unused_imports)]
use std::str::FromStr;
#[allow(unused_imports)]
use void_codex::{
    Direction, InputValue, Objects, Tiles, BRAND_36_BITS, BURDENS_4_BITS, MAX_AUTHOR_LEN,
    MAX_DESCRIPTION_LEN, MAX_NAME_LEN,
};

/// The error type containing all possible fail-states of Void Voyager.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// POST, PUT: The level's data was invalid. See [`void_codex::error::Error`] for details.
    #[error(transparent)]
    InvalidLevel(#[from] void_codex::error::Error),
    /// POST, PUT: A level with an identical name and author has already been uploaded.
    #[error("a level by that name and by that author has already been uploaded")]
    LevelNameCollision,
    /// GET, POST, PUT, DELETE: The key was valid, but a matching
    /// level was not found.
    ///
    /// For GET, this is the level check that
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
    /// does on startup (checking that all stored keys are in the Void
    /// Voyager database). For POST, this is the anti-orphan check that
    /// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
    /// does soon after sending a level upload request, to make sure that the
    /// client received the key (to prevent orphan levels in the database). For
    /// PUT and DELETE, this is simply if the database has no matching level.
    #[error("level not found")]
    LevelNotFound,
    /// The user was manually banned through the Web UI and is no longer allowed to
    /// upload levels.
    #[error("you have been banned")]
    Banned,
    /// The given IP adress to ban by use of the Web UI was invalid.
    #[error("invalid ip")]
    InvalidIp(#[from] std::net::AddrParseError),
    /// An error occured during password hashing for Web UI login.
    #[error("tokio task join error in webui: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    /// The saved Web UI user is invalid.
    #[error("invalid web ui user")]
    WebUI,
    /// On startup, Void Voyager could not bind to the port
    /// 3000, or could not create the `voyager` directory.
    ///
    /// Most likely, another application is using the port,
    /// or the user does not have write permissions in
    /// the current directory (or is out of storage?).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// On startup, an error occurred when asking for a username
    /// and password for the Web UI (probably a user interrupt).
    #[error("inquire error: {0}")]
    Inquire(#[from] inquire::InquireError),
    /// On startup, an error occurred when setting up
    /// a file watcher for hot reloading the config.
    #[error("config watch error: {0}")]
    Watch(#[from] notify_debouncer_mini::notify::Error),
    #[error("bincode (de)serialization error: {0}")]
    /// On startup, the level database could not be serialized.
    ///
    /// Most likely, a breaking change has happened (please
    /// report it!), or the file is corrupted.
    Bincode(#[from] bincode::Error),
    /// On startup, the config could not be serialized.
    ///
    /// Most likely, a breaking change has happened (please
    /// report it!), or the file is corrupted.
    #[error("ron deserialization error: {0}")]
    Ron(#[from] ron::de::SpannedError),
}

use axum::http::StatusCode;

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let status = match self {
            Self::LevelNotFound => StatusCode::NOT_FOUND,
            Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Banned => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, message).into_response()
    }
}
