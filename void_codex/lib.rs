//! Void Codex contains everything needed for working with levels from
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).

pub mod error;
mod level;
pub mod parser;
mod prelude;

pub use level::Author;
pub use level::Brand;
pub use level::BrandImage;
pub use level::BranefuckProgram;
pub use level::Burdens;
pub use level::Data;
pub use level::Description;
pub use level::Direction;
pub use level::Edited;
pub use level::IndexLevel;
pub use level::InputValue;
pub use level::Key;
pub use level::Level;
pub use level::Message;
pub use level::Multiplier;
pub use level::Music;
pub use level::Name;
pub use level::Object;
pub use level::ObjectId;
pub use level::ObjectType;
pub use level::Objects;
pub use level::Parsed;
pub use level::ParsedBurdens;
pub use level::ParsedObjects;
pub use level::ParsedTiles;
pub use level::Tile;
pub use level::TileId;
pub use level::TileType;
pub use level::Tiles;
pub use level::Unvalidated;
pub use level::Uploaded;
pub use level::Validated;
pub use level::Version;
pub use level::BRAND_36_BITS;
pub use level::BRANEFUCK_CHARACTERS;
pub use level::BURDENS_4_BITS;
pub use level::MAX_AUTHOR_LEN;
pub use level::MAX_DESCRIPTION_LEN;
pub use level::MAX_NAME_LEN;
