use axum::http::StatusCode;
use tracing::{info, warn};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid request structure")]
    InvalidStructure,
    #[error("invalid format version: {0}")]
    InvalidVersion(NumberError),
    #[error("invalid name: {0}")]
    InvalidName(StringError),
    #[error("invalid description: {0}")]
    InvalidDescription(StringError),
    #[error("invalid music: {0}")]
    InvalidMusic(StringError),
    #[error("invalid music: not a valid song")]
    NotASong,
    #[error("invalid author: {0}")]
    InvalidAuthor(StringError),
    #[error("invalid brand: {0}")]
    InvalidBrand(NumberError),
    #[error("invalid burdens: {0}")]
    InvalidBurdens(NumberError),
    #[error("invalid tiles")]
    InvalidTiles,
    #[error("invalid objects")]
    InvalidObjects,
    #[error("key error: {0}")]
    InvalidKey(#[from] ulid::DecodeError),
    #[error("level not found")]
    LevelNotFound,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bincode (de)serialization error: {0}")]
    Bincode(#[from] bincode::Error),
}

#[derive(thiserror::Error, Debug)]
// i don't want an enum with the name "String" lol
#[allow(clippy::module_name_repetitions)]
pub enum StringError {
    #[error("invalid base64")]
    Base64(#[from] base64::DecodeError),
    #[error("utf8 error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("input was too long: {found} > {max}")]
    TooLong { max: u64, found: u64 },
    #[error("input was too short: 0 < 1")]
    TooShort,
}

#[derive(thiserror::Error, Debug)]
// i don't want an enum with the name "Number" lol
#[allow(clippy::module_name_repetitions)]
pub enum NumberError {
    #[error("not a number: {0}")]
    NotANumber(#[from] std::num::ParseIntError),
    #[error("too big of a number: {found} > {max}")]
    TooBig { max: u64, found: u64 },
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
