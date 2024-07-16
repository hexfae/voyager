//! Common items used throughout Void Codex.
pub use crate::error::Error;
pub use crate::error::NumberError;
pub use crate::error::StringError;
/// The common result type used throughout Void
/// Codex, using Void Codex's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
