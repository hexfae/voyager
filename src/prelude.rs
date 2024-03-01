pub use crate::error::Error;
pub use crate::utils::level::Level;
pub use crate::utils::server::SharedAppState;
pub type Result<T> = core::result::Result<T, Error>;
