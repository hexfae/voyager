//! Common items used throughout Voyager.
pub use crate::error::Error;
pub use crate::error::NumberError;
pub use crate::error::StringError;
pub use crate::utils::level::Key;
pub use crate::utils::level::Level;
pub use crate::utils::level::Parsed;
pub use crate::utils::server::Backend;
pub use crate::utils::server::Credentials;
pub use crate::utils::server::SharedAppState;
/// The common result type used throughout
/// Voyager, using Voyager's [`Error`].
pub type AuthSession = axum_login::AuthSession<Backend>;
pub type Result<T> = std::result::Result<T, Error>;
