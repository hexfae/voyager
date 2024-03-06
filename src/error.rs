//! Voyager's comprehensive error type.
//!
//! Notably, Voyager does not have (need)
//! a "catch-all" generic error variant.

use axum::http::StatusCode;
use tracing::{info, warn};

// for documentation
#[allow(unused_imports)]
use crate::utils::level::{
    BLACK_HOLE_FORMAT, BRAND_36_BITS, BURDENS_4_BITS, MAX_AUTHOR_LEN, MAX_DESCRIPTION_LEN,
    MAX_NAME_LEN, VALID_MUSIC,
};

/// The main error type, containing all possible fail-states of Voyager.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// POST and PUT: The input data was invalid. Either it had
    /// too many separators, not enough separators, or was otherwise
    /// incorrect. Input data should contain 11 separators ('|') for
    /// POST and 12 separators for PUT.
    #[error("invalid request structure")]
    InvalidStructure,
    /// POST and PUT: The format version was invalid. Either it
    /// was not a number, was too small (0), or was too big (>1).
    /// Current format version is 1.
    #[error("invalid format version: {0}")]
    InvalidVersion(NumberError),
    /// POST and PUT: The name was invalid. Either it was invalid
    /// Base64, was too small (0), was too big (>[`MAX_NAME_LEN`]),
    /// or was somehow decoded into invalid UTF-8.
    #[error("invalid name: {0}")]
    InvalidName(StringError),
    /// POST and PUT: The description was invalid. Either it was
    /// invalid Base64, was too big (>[`MAX_DESCRIPTION_LEN`]),
    /// or was somehow decoded into invalid UTF-8.
    #[error("invalid description: {0}")]
    InvalidDescription(StringError),
    /// POST and PUT: The music was invalid. Either it was invalid
    /// Base64, or was somehow decoded into invalid UTF-8.
    #[error("invalid music: {0}")]
    InvalidMusic(StringError),
    /// POST and PUT: The music was invalid. The song was not in [`VALID_MUSIC`].
    #[error("invalid music: not a valid song")]
    NotASong,
    /// POST and PUT: The author was invalid. Either it was invalid
    /// Base64, was too small (0), was too big (>[`MAX_AUTHOR_LEN`]),
    /// or was somehow decoded into invalid UTF-8.
    #[error("invalid author: {0}")]
    InvalidAuthor(StringError),
    /// POST and PUT: The author brand was invalid. Either it was
    /// not a number, or was too big ([`BRAND_36_BITS`]).
    #[error("invalid brand: {0}")]
    InvalidBrand(NumberError),
    /// POST and PUT: The level burdens were invalid. Either it was
    /// not a number, or was too big ([`BURDENS_4_BITS`]).
    #[error("invalid burdens: {0}")]
    InvalidBurdens(NumberError),
    /// POST and PUT: The level tiles were invalid. One or more
    /// characters were not in ([`BLACK_HOLE_FORMAT`]).
    #[error("invalid tiles")]
    InvalidTiles,
    /// POST and PUT: The level objects were invalid. One or more
    /// characters were not in ([`BLACK_HOLE_FORMAT`]).
    #[error("invalid objects")]
    InvalidObjects,
    /// PUT and DELETE: The key was invalid. The key could not be
    /// parsed into a [ULID](https://github.com/ulid/spec) key.
    #[error("key error: {0}")]
    InvalidKey(#[from] ulid::DecodeError),
    /// GET, POST, PUT, DELETE: The key was valid, but a matching
    /// level was not found. For GET, this is the level check that
    /// Endless Void does on startup (checking that all stored keys
    /// are in the Voyager database). For POST, this is the anti-
    /// orphan check that Endless Void does soon after sending a
    /// level upload request, to make sure that the client received
    /// the key (to prevent orphan levels in the database). For PUT
    /// and DELETE, this is simply if the database has no matching level.
    #[error("level not found")]
    LevelNotFound,
    /// On startup, if Voyager could not bind to the port 3000.
    /// Most likely, another application is using it.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// On startup, if Voyager could not deserialize the saved
    /// file (`./voyager.db`) containing the stored levels.
    #[error("bincode (de)serialization error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("inquire error: {0}")]
    Inquire(#[from] inquire::InquireError),
}

/// All number-related Voyager errors.
#[derive(thiserror::Error, Debug)]
// i don't want an enum with the name "Number" lol
#[allow(clippy::module_name_repetitions)]
pub enum NumberError {
    /// POST and PUT. Number could not
    /// be parsed (was invalid).
    #[error("not a number: {0}")]
    NotANumber(#[from] std::num::ParseIntError),
    /// POST and PUT. Number was too big.
    #[error("too big of a number: {found} > {max}")]
    TooBig {
        ///Version must (currently) be 1. Author brand must
        /// be between 0 and [`BRAND_36_BITS`].
        ///
        /// Burdens must be between 0 and [`BURDENS_4_BITS`].
        max: u64,
        /// What the user's input was.
        found: u64,
    },
}

/// All string-related Voyager errors.
#[derive(thiserror::Error, Debug)]
// i don't want an enum with the name "String" lol
#[allow(clippy::module_name_repetitions)]
pub enum StringError {
    /// POST and PUT: Input was invalid Base64. Name, description,
    /// music, and author should be Base64-encoded.
    #[error("invalid base64")]
    Base64(#[from] base64::DecodeError),
    /// POST and PUT: Input was invalid UTF-8. Name, description,
    /// music, and author should be valid UTF-8.
    #[error("utf8 error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// POST and PUT: Input was too long.
    #[error("input was too long: {found} > {max}")]
    TooLong {
        /// Name may at most be [`MAX_NAME_LEN`] long.
        ///
        /// Author may at most be [`MAX_AUTHOR_LEN`] long.
        ///
        /// Description may at most be [`MAX_DESCRIPTION_LEN`] long.
        max: u64,
        /// What the user's input was.
        found: u64,
    },
    /// POST and PUT: Input was too short. Name and
    /// author must be at least 1 character long.
    #[error("input was too short: 0 < 1")]
    TooShort,
}

impl axum::response::IntoResponse for Error {
    // unavoidable big match statement
    #[allow(clippy::cognitive_complexity)]
    fn into_response(self) -> axum::response::Response {
        let message = self.to_string();
        let status = match self {
            Self::LevelNotFound => {
                info!("{self}");
                StatusCode::NOT_FOUND
            }
            Self::Bincode(why) => {
                warn!("{why}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Io(why) => {
                warn!("{why}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            other => {
                info!("{other}");
                StatusCode::BAD_REQUEST
            }
        };
        (status, message).into_response()
    }
}
