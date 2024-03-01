use axum::http::StatusCode;
use tracing::{info, warn};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid request structure")]
    InvalidStructure,
    #[error("invalid format version")]
    InvalidVersion,
    #[error("invalid name")]
    InvalidName,
    #[error("invalid description")]
    InvalidDescription,
    #[error("invalid music choice")]
    InvalidMusic,
    #[error("invalid author")]
    InvalidAuthor,
    #[error("invalid brand")]
    InvalidBrand,
    #[error("invalid burdens")]
    InvalidBurdens,
    #[error("invalid tiles")]
    InvalidTiles,
    #[error("invalid objects")]
    InvalidObjects,
    #[error("invalid key")]
    InvalidKey(#[from] ulid::DecodeError),
    #[error("level not found")]
    LevelNotFound,
    #[error("internal server error")]
    InternalServerError,
    #[error("i/o error")]
    IOError(#[from] std::io::Error),
    #[error("invalid stored level")]
    InvalidLevel(#[from] std::string::FromUtf8Error),
    #[error("bincode (de)serialization error")]
    Bincode(#[from] bincode::Error),
}

impl axum::response::IntoResponse for Error {
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
            Self::IOError(why) => {
                warn!("{why}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::InternalServerError => {
                warn!("internal server error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::BAD_REQUEST,
        };
        (status, message).into_response()
    }
}
