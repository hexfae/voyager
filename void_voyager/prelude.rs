//! Common items used throughout Void Voyager.
pub use crate::error::Error;
pub use void_codex::error::Error as LevelError;
/// Authentication session used for
/// logging in to the Web UI.
pub type AuthSession = axum_login::AuthSession<crate::webui::backend::Backend>;
/// The common result type used throughout Void
/// Voyager, using Void Voyager's [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
/// Thread-safe app state, used across Void Voyager.
pub type SharedAppState = std::sync::Arc<crate::server::AppState>;
