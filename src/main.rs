//! Voyager is the server back-end for
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void),
//! which is a level editor for
//! [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/),
//! a "2D sokoban-style puzzle game where every step counts."
//!
//! It supports uploading levels, editing levels, deleting levels, and
//! downloading all uploaded levels. Authentication is managed through
//! a per-level key-based system.

mod error;
mod prelude;
mod utils;

#[tokio::main]
async fn main() -> prelude::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Voyager is launching.");
    utils::server::start_voyager().await
}
