//! Routers for GET, POST, PUT, and DELETE, and a fallback router.

pub mod delete;
pub mod get;
pub mod post;
pub mod put;
pub mod teapot;
#[cfg(test)]
pub mod tests;
pub mod version;
