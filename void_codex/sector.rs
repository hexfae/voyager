//! Contains the [`Level`] struct, its [`Unvalidated`]
//! and [`Validated`] states, and related constants.

use crate::prelude::*;

use crate::error::Error;
use crate::parser::{parse_objects, parse_tiles};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::{Engine, prelude::BASE64_STANDARD};
use bitvec::order::Lsb0;
use bitvec::view::BitView;
use derive_more::Display;
use image::imageops::{FilterType, resize};
use image::{ImageBuffer, ImageFormat, Rgb};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use std::{convert::Infallible, io::Cursor, net::IpAddr};
use time::OffsetDateTime;
use tracing::warn;
use ulid::Ulid;
use unicode_segmentation::UnicodeSegmentation;

const VOID_STRANGER_LEVEL_WIDTH: usize = 14;

/// A level's author's brand's highest value.
///
/// Equal to 2^36-1, 68719476735, or `68_719_476_735`.
pub const BRAND_36_BITS: u64 = 0b1111_1111_1111_1111_1111_1111_1111_1111_1111;

/// All of the valid
/// [Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// characters.
///
/// These are the standard
/// [Brainfuck](https://en.wikipedia.org/wiki/Brainfuck)
/// characters, minus `,` (input is instead given in the level editor's
/// UI), plus `?` (returns a number corresponding to the sign of the
/// current cell's number), plus all decimal digits (since they may
/// be used as multipliers, e.g. `+5` instead of `+++++`).
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck) for details.
pub const BRANEFUCK_CHARACTERS: &str = "<>+-.[]?1234567890";

/// A level's burdens' highest value.
///
/// Equal to 2^4-1 or 15.
pub const BURDENS_4_BITS: u8 = 0b1111;

/// A level's author's max length.
pub const MAX_AUTHOR_LEN: usize = 30;

/// A level's decription's max length.
pub const MAX_DESCRIPTION_LEN: usize = 256;

/// A level's name's max length.
pub const MAX_NAME_LEN: usize = 30;

// TODO: a level pack is called a cluster of sectors? or maybe a domain
/// A Void Stranger level.
#[derive(Debug, Clone, Display, Serialize, Deserialize)]
#[display("{cipher}")]
pub struct Sector {
    sigil: Sigil,
    cipher: Cipher,
    compendium: Compendium,
    origin: IpAddr,
}

#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Sigil(Ulid);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Cipher(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compendium {
    pub version: Version,
    pub name: Name,
    pub description: Description,
    pub music: Music,
    pub author: Author,
    pub brand: Brand,
    pub brand_image: BrandImage,
    pub uploaded: Uploaded,
    pub edited: Edited,
    pub burdens: Burdens,
    pub parsed_burdens: ParsedBurdens,
    pub tiles: Tiles,
    pub parsed_tiles: ParsedTiles,
    pub objects: Objects,
    pub parsed_objects: ParsedObjects,
}

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Version(u8);

#[derive(Debug, Display, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Name(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Description(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Music(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Author(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Brand(u64);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display("{base64}")]
pub struct BrandImage {
    pub base64: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Uploaded(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Edited(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(u8);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ParsedBurdens {
    memory: bool,
    wings: bool,
    sword: bool,
    stack_rod: bool,
}

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTiles(Vec<Tile>);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedObjects(Vec<Object>);

/// A tile, as represented by [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// Tiles are typically static pieces of the environment, such as walls, floors,
/// and pits. However, they may also be bombs, the exit, a floor switch, or
/// anything else found in [`TileId`].
///
/// A tile is encoded as `id[type][multiplier]`. For example, a pit is `pt`. A
/// wall of type 3 is `wa03`. Floor repeated 7 times is `flX7`. A wall of type
/// 3 repeated 4 times is `wa03X4`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    /// This tile's id.
    ///
    /// See [`TileId`] for details.
    pub id: TileId,
    /// This tile's (optional) type.
    ///
    /// See [`TileType`] for details.
    // type is a reserved name, and typing r#type is ugly
    #[allow(clippy::struct_field_names)]
    pub tile_type: Option<TileType>,
    /// This tile's (optional) multiplier.
    ///
    /// See [`Multiplier`] for details.
    pub multiplier: Option<Multiplier>,
}

/// All valid tile IDs.
///
/// Every ID is encoded as 2 lowercase characters, e.g. `wa` means wall.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TileId {
    /// Encoded as `pt`.
    Pit,
    /// Encoded as `fl`.
    Floor,
    /// Encoded as `gl`.
    Glass,
    /// Encoded as `mn`.
    Bomb,
    /// Encoded as `xp`.
    LitBomb,
    /// Encoded as `fs`.
    FloorSwitch,
    /// Encoded as `cr`.
    CopyFloor,
    /// Encoded as `ex`.
    Exit,
    /// Encoded as `df`.
    DeathFloor,
    /// Encoded as `bl`.
    BlackFloor,
    /// Encoded as `wh`.
    BlankFloor,
    /// Encoded as `wa`.
    Wall,
    /// Encoded as `mw`.
    FunhouseWall,
    /// Encoded as `dw`.
    DISWall,
    /// Encoded as `ew`.
    EXWall,
    /// Encoded as `ed`.
    Edge,
    /// Encoded as `de`.
    DISEdge,
    /// Encoded as `st`.
    SmallChest,
}

/// A tile's (optional) type.
///
/// Types are encoded as a decimal number, and are always padded to
/// 2 digits, e.g. `9` becomes `09`.
///
/// Two example of tiles with a type are walls (corners, orientation).
/// and small chests (loot).
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display("{_0:02}")] // padded, e.g. 9 -> 09
pub struct TileType(pub u8);

/// An object, as represented by [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// The list of objects is comprised mainly of enemies and statues. However, 2
/// notable exceptions are the player and the secret exit.
///
/// An object is encoded as `id[type][multiplier]`. For example, the player is `pl`.
/// A wall of type 3 is `wa03`. Floor repeated 7 times is `flX7`. A wall of type
/// 3 repeated 4 times is `wa03X4`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// This object's id.
    ///
    /// See [`ObjectId`] for details.
    pub id: ObjectId,
    /// This object's (optional) type.
    ///
    /// See [`ObjectType`] for details.
    // type is a reserved name, and typing r#type is ugly
    #[allow(clippy::struct_field_names)]
    pub object_type: Option<ObjectType>,
    /// This objects's (optional) multiplier.
    ///
    /// See [`Multiplier`] for details.
    pub multiplier: Option<Multiplier>,
}

/// All valid object IDs.
///
/// Every ID is encoded as 2 lowercase characters, e.g. `pl` means player.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ObjectId {
    /// Encoded as `em`.
    Empty,
    /// Encoded as `pl`.
    Player,
    /// Encoded as `cl`.
    Leech,
    /// Encoded as `cc`.
    Maggot,
    /// Encoded as `cg`.
    Beaver,
    /// Encoded as `cs`.
    Smile,
    /// Encoded as `ch`.
    Eye,
    /// Encoded as `cm`.
    Mimic,
    /// Encoded as `co`.
    Octahedron,
    /// Encoded as `hu`.
    FamishedMan,
    /// Encoded as `ad`.
    AddStatue,
    /// Encoded as `cf`.
    CifStatue,
    /// Encoded as `be`.
    BeeStatue,
    /// Encoded as `tn`.
    TanStatue,
    /// Encoded as `lv`.
    LevStatue,
    /// Encoded as `mo`.
    MonStatue,
    /// Encoded as `eu`.
    EusStatue,
    /// Encoded as `go`.
    GorStatue,
    /// Encoded as `jb`.
    Jukebox,
    /// Encoded as `eg`.
    Egg,
    /// Encoded as `ho`.
    FakeEgg,
    /// Encoded as `mm`.
    MemoryCrystal,
    /// Encoded as `se`.
    SecretExit,
    /// Encoded as `ct`.
    Spider,
    /// Encoded as `sd`.
    Scaredeer,
    /// Encoded as `cv`.
    OrbThing,
}

/// An object's type.
///
/// There are 4 different types of object types, see
/// their respective documentation for details:
/// 1. [`Self::Direction`]
/// 2. [`Self::AddStatue1`]
/// 3. [`Self::AddStatue2`]
/// 4. [`Self::Egg`]
///
/// Note: An enemy with a direction of up (e.g. `ct0`)
/// will get its type parsed as [`ObjectType::Egg`] with 0
/// messages, instead of [`ObjectType::Direction`] with
/// [`Direction::Up`]. This is fine, however, because both
/// will be encoded as `0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    /// This object's [`Direction`].
    Direction {
        /// See [`Direction`].
        direction: Direction,
    },
    /// A type 1 Add statue's parameters.
    ///
    /// This takes in 2 [`InputValue`]s, see its documentation for details.
    AddStatue1 {
        /// See [`InputValue`].
        first_input: InputValue,
        /// See [`InputValue`].
        destroy_value: InputValue,
    },
    /// A type 2 Add statue's parameters.
    ///
    /// This takes in 3 [`InputValue`]s and a [`BranefuckProgram `]
    /// program. See their documentation for details.
    AddStatue2 {
        /// See [`InputValue`].
        first_input: InputValue,
        /// See [`InputValue`].
        second_input: InputValue,
        /// See [`InputValue`].
        destroy_value: InputValue,
        /// See [`BranefuckProgram`].
        branefuck: BranefuckProgram,
    },
    /// An egg's messages.
    ///
    /// An egg has a list of messages that will be displayed when interacted with (?). The
    /// length may be (and is often) 0. The longest length found in the wild is 4 (the max?).
    Egg {
        /// See [`ObjectType::Egg`].
        messages: Vec<Message>,
    },
}

/// A tile's (optional) multiplier.
///
/// Multipliers are prefixed with `X`. They represent how many times a
/// tile or object is repeated in a row. For example, instead of `flflflflfl`,
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// encodes 5 floor tiles in a row as `flX5`.
#[derive(Debug, Default, Display, Clone, Serialize, Deserialize)]
#[display("X{_0}")] // X prefix, e.g. 9 -> X9
pub struct Multiplier(pub u8);

/// An object's direction.
///
/// Encoded as a number between `0` and `3`. The reasoning behind these is
/// "it's the same order of directions used in math for angles 0-360."
///
/// This is only used for enemies, as far as I can tell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    /// Encoded as `0`.
    Right,
    /// Encoded as `1`.
    Up,
    /// Encoded as `2`.
    Left,
    /// Encoded as `3`.
    Down,
}

/// All valid input values for Add statues.
///
/// Valid inputs are any number, any
/// [Endless Void global variable](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck#input-variables),
/// and any vanilla global variable.
///
/// Listed are [all of the global variables Endless Void introduced](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck#input-variables).
/// Since I haven't yet decompiled the game to check the vanilla global
/// variables, the [`InputValue::Unknown`] variant wraps a [`String`].
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// for details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputValue {
    /// Unknown input, either a number or a vanilla global variable.
    ///
    /// Since I don't know all of the available vanilla global variables,
    /// and since a number is a valid input, this variant wraps a [`String`].
    Unknown(String),
    /// Global variable, encoded as `leech_count`.
    LeechCount,
    /// Global variable, encoded as `maggot_count`.
    MaggotCount,
    /// Global variable, encoded as `beaver_count`.
    BeaverCount,
    /// Global variable, encoded as `smile_count`.
    SmileCount,
    /// Global variable, encoded as `eye_count`.
    EyeCount,
    /// Global variable, encoded as `mimic_count`.
    MimicCount,
    /// Global variable, encoded as `octahedron_count`.
    OctahedronCount,
    /// Global variable, encoded as `spider_count`.
    SpiderCount,
    /// Global variable, encoded as `orb_count`.
    OrbCount,
    /// Global variable, encoded as `scaredeer_count`.
    ScaredeerCount,
    /// Global variable, encoded as `player_x`.
    ///
    /// This is between `0` and `13`.
    PlayerX,
    /// Global variable, encoded as `player_y`.
    ///
    /// This is between `0` and `8`.
    PlayerY,
    /// Global variable, encoded as `editor_time`.
    EditorTime,
    /// Global variable, encoded as `add_count`.
    AddCount,
    /// Global variable, encoded as `mon_count`.
    MonCount,
    /// Global variable, encoded as `tan_count`.
    TanCount,
    /// Global variable, encoded as `lev_count`.
    LevCount,
    /// Global variable, encoded as `eus_count`.
    EusCount,
    /// Global variable, encoded as `bee_count`.
    BeeCount,
    /// Global variable, encoded as `gor_count`.
    GorCount,
    /// Global variable, encoded as `cif_count`.
    CifCount,
    /// Global variable, encoded as `jukebox_count`.
    JukeboxCount,
    /// Global variable, encoded as `egg_count`.
    EggCount,
}

/// A [Branefuck program](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck).
///
/// Add statues are able to run
/// [Branefuck programs](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// in [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// See [`BRANEFUCK_CHARACTERS`] for details on valid characters.
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck) for further details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct BranefuckProgram(String);

/// An egg's message.
///
/// See [`ObjectType::Egg`] for details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Message(pub String);

impl Sector {
    pub fn inscribe(
        cipher: impl Into<String> + AsRef<str>,
        ip: IpAddr,
        latest_version: impl Into<u8>,
        allowed_songs: impl AsRef<[String]>,
    ) -> Result<Self> {
        Ok(Self {
            sigil: Sigil::new(),
            compendium: Compendium::decipher(cipher.as_ref(), latest_version, allowed_songs)?,
            cipher: Cipher(cipher.into()),
            origin: ip,
        })
    }

    pub fn amend(
        cipher: impl Into<String> + AsRef<str>,
        sigil: Sigil,
        ip: IpAddr,
        latest_version: impl Into<u8>,
        allowed_songs: impl AsRef<[String]>,
    ) -> Result<Self> {
        Ok(Self {
            sigil,
            compendium: Compendium::decipher(cipher.as_ref(), latest_version, allowed_songs)?,
            cipher: Cipher(cipher.into()),
            origin: ip,
        })
    }

    pub fn set_dates_to_now(&mut self) -> Result<()> {
        let (version, name, description, music, author, brand, _, _, burdens, tiles, objects) =
            self.cipher
                .0
                .splitn(11, '|')
                .collect_tuple()
                .ok_or(Error::InvalidStructure)?;

        let now = OffsetDateTime::now_utc()
            // 2025-03-02
            .date()
            .to_string()
            // 20250302
            .replace('-', "");
        self.cipher = Cipher(format!(
            "{version}|{name}|{description}|{music}|{author}|{brand}|{now}|{now}|{burdens}|{tiles}|{objects}"
        ));
        self.compendium.uploaded = Uploaded(now.clone());
        self.compendium.edited = Edited(now);
        Ok(())
    }

    pub fn set_uploaded_from(&mut self, sector: &Self) -> Result<()> {
        let uploaded = sector.compendium.uploaded.clone();
        let (version, name, description, music, author, brand, _, edited, burdens, tiles, objects) =
            self.cipher
                .0
                .splitn(11, '|')
                .collect_tuple()
                .ok_or(Error::InvalidStructure)?;

        self.cipher = Cipher(format!(
            "{version}|{name}|{description}|{music}|{author}|{brand}|{uploaded}|{edited}|{burdens}|{tiles}|{objects}"
        ));
        self.compendium.uploaded = uploaded;
        Ok(())
    }

    /// Converts its parsed tiles and parsed objects to emojis.
    ///
    /// First, it calls [`ParsedTiles::to_emojis`] and [`ParsedObjects::to_emojis`].
    /// Then, it takes an iterator over both's graphemes (like characters, but treating
    /// characters with multiple Unicode codepoints as one, e.g. emojis) and zips them up
    /// into an iterator of pairs. Going through every pair, if the object is "❌" (empty
    /// or secret exit), it uses the tile emoji, else it uses the object emoji, meaning
    /// objects go on top of tiles. It then replaces every placeholder character with its
    /// full representation (read below).
    ///
    /// In the hud tiles, there are a few tiles that can't be represented as a single emoji,
    /// those being "VO", "ID", "00", "V?", and "??". Fortunately, in discord code blocks,
    /// emojis are twice as wide as letters, letting us instead just use the characters.
    /// Unfortunately, having two characters in place of a single emoji messes with the
    /// length of the map. Instead, a placeholder is used for each of these tiles, those
    /// being 'O', 'D', '0', 'V', and '?'. Thus, an object (emoji) can replace one of these
    /// placeholders, and whichever placeholders are remaining will be swapped out for their
    /// full-length representations, e.g. 'O' -> "VO", before being sent.
    #[must_use]
    pub fn to_emoji_map(&self) -> String {
        let tiles = self
            .compendium
            .parsed_tiles
            .to_emoji_map(&self.compendium.parsed_burdens);
        let tiles = tiles.graphemes(true);
        let objects = self.compendium.parsed_objects.to_emoji_map();
        let objects = objects.graphemes(true);
        let map = tiles
            .zip(objects)
            .map(|(tile, object)| if object == "❌" { tile } else { object })
            .collect::<String>();

        // do this one first...
        let map = map.replace('?', "??");
        // ...so that it doesn't replace the ? from this one
        // and do this one second...
        let map = map.replace('V', "V?");
        // ...so that it doesn't replace the V from this one
        let map = map.replace('O', "VO");
        let map = map.replace('D', "ID");
        map.replace('0', "00")
    }

    #[must_use]
    pub fn version(&self) -> String {
        self.compendium.version.to_string()
    }

    #[must_use]
    pub fn name(&self) -> String {
        self.compendium.name.to_string()
    }

    #[must_use]
    pub fn description(&self) -> String {
        self.compendium.description.to_string()
    }

    #[must_use]
    pub fn music(&self) -> String {
        self.compendium.music.to_string()
    }

    #[must_use]
    pub fn author(&self) -> String {
        self.compendium.author.to_string()
    }

    #[must_use]
    pub fn brand(&self) -> String {
        self.compendium.brand.to_string()
    }

    #[must_use]
    pub fn brand_image_base64(&self) -> String {
        self.compendium.brand_image.base64.clone()
    }

    #[must_use]
    pub fn brand_image_bytes(&self) -> Vec<u8> {
        self.compendium.brand_image.bytes.clone()
    }

    #[must_use]
    pub fn uploaded(&self) -> String {
        self.compendium.uploaded.to_string()
    }

    #[must_use]
    pub fn edited(&self) -> String {
        self.compendium.edited.to_string()
    }

    #[must_use]
    pub fn burdens(&self) -> String {
        self.compendium.burdens.to_string()
    }

    #[must_use]
    pub fn parsed_burdens(&self) -> ParsedBurdens {
        self.compendium.parsed_burdens.clone()
    }

    #[must_use]
    pub fn sigil(&self) -> String {
        self.sigil.to_string()
    }

    #[must_use]
    pub fn cipher(&self) -> String {
        self.cipher.to_string()
    }

    #[must_use]
    pub const fn origin(&self) -> IpAddr {
        self.origin
    }
}

impl Sigil {
    /// Generates a new [ULID](https://github.com/ulid/spec)
    /// key for a level.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Compendium {
    pub fn decipher(
        cipher: impl AsRef<str>,
        latest_version: impl Into<u8>,
        allowed_songs: impl AsRef<[String]>,
    ) -> Result<Self> {
        let (
            version,
            name,
            description,
            music,
            author,
            brand,
            uploaded,
            edited,
            burdens,
            tiles,
            objects,
        ) = cipher
            .as_ref()
            .splitn(11, '|')
            .collect_tuple()
            .ok_or(Error::InvalidStructure)?;

        let version = Version::from_str(version, latest_version)?;
        let name = name.parse()?;
        let description = description.parse()?;
        let music = Music::from_str(music, allowed_songs)?;
        let author = author.parse()?;
        let brand = brand.parse()?;
        let brand_image = BrandImage::new(&brand);
        let uploaded = Uploaded(uploaded.to_string());
        let edited = Edited(edited.to_string());
        let burdens = burdens.parse()?;
        let parsed_burdens = ParsedBurdens::from(&burdens);
        let parsed_tiles = tiles.parse()?;
        let tiles = tiles.parse()?;
        let parsed_objects = objects.parse()?;
        let objects = objects.parse()?;

        Ok(Self {
            version,
            name,
            description,
            music,
            author,
            brand,
            brand_image,
            uploaded,
            edited,
            burdens,
            parsed_burdens,
            tiles,
            parsed_tiles,
            objects,
            parsed_objects,
        })
    }
}

impl TileId {
    fn to_emoji(&self) -> String {
        match self {
            Self::Pit | Self::Edge | Self::DISEdge => "⬛️",
            Self::Floor | Self::BlankFloor => "⬜️",
            Self::Glass => "🪟",
            Self::Bomb | Self::LitBomb => "💣️",
            Self::FloorSwitch => "⏹️",
            Self::CopyFloor => "🔯",
            Self::Exit => "🚪",
            Self::DeathFloor => "🟥",
            Self::BlackFloor => "🔲",
            Self::Wall | Self::FunhouseWall | Self::DISWall | Self::EXWall => "🧱",
            Self::SmallChest => "📦",
        }
        .to_string()
    }
}

impl ObjectId {
    fn to_emoji(&self) -> String {
        match self {
            Self::Empty | Self::SecretExit => "❌",
            Self::Player => "🧑",
            Self::Leech => "🐍",
            Self::Maggot => "🐛",
            Self::Beaver => "🦫",
            Self::Smile => "😄",
            Self::Eye => "🖐️",
            Self::Mimic => "⛄️",
            Self::Octahedron => "🔷",
            Self::FamishedMan => "🧟",
            Self::AddStatue
            | Self::CifStatue
            | Self::BeeStatue
            | Self::TanStatue
            | Self::LevStatue
            | Self::MonStatue
            | Self::EusStatue
            | Self::GorStatue => "🗿",
            Self::Jukebox => "📻️",
            Self::Egg | Self::FakeEgg => "🪨",
            Self::MemoryCrystal => "✨",
            Self::Spider => "🕷️",
            Self::Scaredeer => "🦌",
            Self::OrbThing => "💡",
        }
        .to_string()
    }
}

impl ParsedTiles {
    fn to_emoji_map(&self, burdens: &ParsedBurdens) -> String {
        let mut map = String::new();
        for tile in &self.0 {
            let emoji = tile.id.to_emoji();
            let mut string = String::new();
            let multiplier = tile.multiplier.clone().unwrap_or_default().0;
            string.push_str(&emoji.clone());
            for _ in 1..multiplier {
                string.push_str(&emoji.clone());
            }
            map.push_str(&string);
        }
        let memory = if burdens.memory { "🧊" } else { "⬜️" };
        let wings = if burdens.wings { "🪽" } else { "⬜️" };
        let sword = if burdens.sword { "🗡️" } else { "⬜️" };
        let rod = if burdens.stack_rod { "🪄" } else { "🪈" };
        let hud_tiles = format!("⬜️OD⬜️🪰0{rod}⬜️{memory}{wings}{sword}⬜️V?");
        map.push_str(&hud_tiles);
        map.graphemes(true)
            .chunks(VOID_STRANGER_LEVEL_WIDTH)
            .into_iter()
            .map(Iterator::collect::<String>)
            .collect_vec()
            .join("\n")
    }
}

impl ParsedObjects {
    fn to_emoji_map(&self) -> String {
        let mut emojis = String::new();
        for object in &self.0 {
            let emoji = object.id.to_emoji();
            let mut string = String::new();
            let multiplier = object.multiplier.clone().unwrap_or_default().0;
            string.push_str(&emoji.clone());
            for _ in 1..multiplier {
                string.push_str(&emoji.clone());
            }
            emojis.push_str(&string);
        }
        emojis
            .graphemes(true)
            .chunks(VOID_STRANGER_LEVEL_WIDTH)
            .into_iter()
            .map(Iterator::collect::<String>)
            .collect_vec()
            .join("\n")
    }
}

impl ObjectType {
    pub fn egg(input: Vec<String>) -> Self {
        let messages = input.into_iter().map(Message).collect();
        Self::Egg { messages }
    }

    // TODO: get rid of this
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_statue(input: Vec<String>) -> Result<Self> {
        match input.len() {
            2 => Ok(Self::AddStatue1 {
                first_input: InputValue::from_str(&input[0])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
                destroy_value: InputValue::from_str(&input[1])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
            }),
            4 => Ok(Self::AddStatue2 {
                first_input: InputValue::from_str(&input[0])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
                second_input: InputValue::from_str(&input[1])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
                destroy_value: InputValue::from_str(&input[2])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
                branefuck: BranefuckProgram::from_str(&input[3])
                    .map_err(|why| Error::InvalidObject(why.to_string()))?,
            }),
            other => Err(Error::InvalidObject(format!(
                "invalid add statue length of {other}"
            ))),
        }
    }

    /// Attempts to parse the input as [`ObjectType::Direction`].
    ///
    /// See [`Direction`] for details.
    ///
    /// # Errors
    ///
    /// Returns an error on invalid input.
    pub fn direction(input: &str) -> Result<Self> {
        Ok(Self::Direction {
            direction: Direction::from_str(input)
                .map_err(|why| Error::InvalidObject(why.to_string()))?,
        })
    }
}

impl FromStr for BranefuckProgram {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        if input.chars().all(|c| BRANEFUCK_CHARACTERS.contains(c)) {
            Ok(Self(input.into()))
        } else {
            Err(Error::InvalidObject(
                "invalid branefuck character found".into(),
            ))
        }
    }
}

impl ParsedTiles {
    pub fn new(tiles: impl Into<Vec<Tile>>) -> Self {
        Self(tiles.into())
    }
}

impl ParsedObjects {
    pub fn new(objects: impl Into<Vec<Object>>) -> Self {
        Self(objects.into())
    }
}

impl Version {
    /// Parses input as an integer for a level's format version
    ///
    /// # Errors
    ///
    /// Returns an error if the input wasn't a number, was too
    /// big, or was too small.
    fn from_str(input: &str, latest_version: impl Into<u8>) -> Result<Self> {
        // TODO: only allow matching versions to be uploaded
        let latest_version = latest_version.into();
        let version = input
            .parse::<u8>()
            .map_err(|why| Error::InvalidVersion(NumberError::NotANumber(why)))?;
        let too_big = version > latest_version;
        let is_zero = version == 0;

        if too_big {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: u64::from(latest_version),
                found: u64::from(version),
            }));
        }
        if is_zero {
            return Err(Error::InvalidVersion(NumberError::TooSmall {
                min: 1,
                found: u64::from(version),
            }));
        }
        Ok(Self(version))
    }
}

impl FromStr for Name {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let name = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidName(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidName(StringError::FromUtf8(why)))?;
        if name.is_empty() {
            return Err(Error::InvalidName(StringError::TooShort));
        }
        if name.len() > MAX_NAME_LEN {
            return Err(Error::InvalidName(StringError::TooLong {
                max: MAX_NAME_LEN as u64,
                found: name.len() as u64,
            }));
        }
        Ok(Self(name))
    }
}

impl FromStr for Description {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let description = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidDescription(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidDescription(StringError::FromUtf8(why)))?;
        if description.len() > MAX_DESCRIPTION_LEN {
            return Err(Error::InvalidDescription(StringError::TooLong {
                max: MAX_DESCRIPTION_LEN as u64,
                found: description.len() as u64,
            }));
        }
        Ok(Self(description))
    }
}

impl Music {
    /// Parses input as music from Void Stranger.
    ///
    /// # Errors
    ///
    /// Returns an error if the input was invalid Base64,
    /// produced invalid UTF-8, or is not one of the allowed songs.
    fn from_str(input: &str, allowed_songs: impl AsRef<[String]>) -> Result<Self> {
        let music = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidMusic(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidMusic(StringError::FromUtf8(why)))?;
        if !allowed_songs.as_ref().contains(&music) {
            return Err(Error::UnknownMusic);
        }
        Ok(Self(music))
    }
}

impl FromStr for Author {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let author = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidAuthor(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidAuthor(StringError::FromUtf8(why)))?;
        if author.is_empty() {
            return Err(Error::InvalidName(StringError::TooShort));
        }
        if author.len() > MAX_AUTHOR_LEN {
            return Err(Error::InvalidName(StringError::TooLong {
                max: MAX_AUTHOR_LEN as u64,
                found: author.len() as u64,
            }));
        }
        Ok(Self(author))
    }
}

impl FromStr for Brand {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let brand = input
            .parse::<u64>()
            .map_err(|why| Error::InvalidBrand(NumberError::NotANumber(why)))?;
        if brand > BRAND_36_BITS {
            return Err(Error::InvalidBrand(NumberError::TooBig {
                max: BRAND_36_BITS,
                found: brand,
            }));
        }
        Ok(Self(brand))
    }
}

impl BrandImage {
    /// Creates a new [`BrandImage`] from a [`Brand`].
    ///
    /// The brand is encoded as a 6x6 PNG image
    /// of black and white pixels in Base64 format.
    pub fn new(input: &Brand) -> Self {
        let width = 6;
        let height = 6;
        let mut img = ImageBuffer::<Rgb<u8>, _>::new(width, height);
        let bits = input.0.view_bits::<Lsb0>();
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let pixel_is_white = bits[(x + y * height) as usize];
            *pixel = if pixel_is_white {
                Rgb([255, 255, 255])
            } else {
                Rgb([0, 0, 0])
            };
        }
        let img = resize(&img, 60, 60, FilterType::Nearest);
        let mut buf = Cursor::new(Vec::new());
        if let Err(why) = img.write_to(&mut buf, ImageFormat::Png) {
            warn!("something went wrong while writing image to buffer! {why}");
        }
        Self {
            bytes: buf.clone().into_inner(),
            base64: BASE64_STANDARD.encode(buf.into_inner()),
        }
    }
}

impl FromStr for Burdens {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        let burdens = input
            .parse::<u8>()
            .map_err(|why| Error::InvalidBurdens(NumberError::NotANumber(why)))?;
        if burdens > BURDENS_4_BITS {
            return Err(Error::InvalidBurdens(NumberError::TooBig {
                max: u64::from(BURDENS_4_BITS),
                found: u64::from(burdens),
            }));
        }
        Ok(Self(burdens))
    }
}

impl From<&Burdens> for ParsedBurdens {
    fn from(input: &Burdens) -> Self {
        let bits = input.0.view_bits::<Lsb0>();
        Self {
            memory: bits[0],
            wings: bits[1],
            sword: bits[2],
            stack_rod: bits[3],
        }
    }
}

impl FromStr for Tiles {
    type Err = Infallible;

    /// Checks the input for validity and returns [`Tiles`].
    ///
    /// See [`ParsedTiles::from_str()`] for details.
    ///
    /// On invalid input, this function will log a warning instead of returning
    /// an error, because I am only 99% confident in the parser.
    fn from_str(input: &str) -> Result<Self, Infallible> {
        // i am only 99% confident in the parser, so for now,
        // only log if an error happens
        if let Err(why) = ParsedTiles::from_str(input) {
            warn!("error while parsing tiles: {why}");
        }
        Ok(Self(input.to_owned()))
    }
}

impl FromStr for TileId {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "pt" => Ok(Self::Pit),
            "fl" => Ok(Self::Floor),
            "gl" => Ok(Self::Glass),
            "mn" => Ok(Self::Bomb),
            "xp" => Ok(Self::LitBomb),
            "fs" => Ok(Self::FloorSwitch),
            "cr" => Ok(Self::CopyFloor),
            "ex" => Ok(Self::Exit),
            "df" => Ok(Self::DeathFloor),
            "bl" => Ok(Self::BlackFloor),
            "wh" => Ok(Self::BlankFloor),
            "wa" => Ok(Self::Wall),
            "mw" => Ok(Self::FunhouseWall),
            "dw" => Ok(Self::DISWall),
            "ew" => Ok(Self::EXWall),
            "ed" => Ok(Self::Edge),
            "de" => Ok(Self::DISEdge),
            "st" => Ok(Self::SmallChest),
            other => Err(Error::InvalidTile(other.to_string())),
        }
    }
}

impl FromStr for ParsedTiles {
    type Err = Error;

    /// Attempts to parse the input as a valid sequence of tiles.
    ///
    /// # Errors
    ///
    /// Returns an error if any tile was invalid. This includes an invalid
    /// [`TileId`], an invalid [`TileType`], an invalid [`Multiplier`],
    /// or some other invalid input.
    fn from_str(input: &str) -> Result<Self> {
        parse_tiles(input)
    }
}

impl FromStr for Objects {
    type Err = Infallible;

    /// Checks the input for validity and returns [`Objects`].
    ///
    /// See [`ParsedObjects::from_str()`] for details.
    ///
    /// On invalid input, this function will log a warning instead of returning
    /// an error, because I am only 99% confident in the parser.
    fn from_str(input: &str) -> Result<Self, Infallible> {
        // i am only 99% confident in the parser, so for now,
        // only log if an error happens
        if let Err(why) = ParsedObjects::from_str(input) {
            warn!("error while parsing objects: {why}");
        }
        Ok(Self(input.to_string()))
    }
}

impl FromStr for ObjectId {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "em" => Ok(Self::Empty),
            "pl" => Ok(Self::Player),
            "cl" => Ok(Self::Leech),
            "cc" => Ok(Self::Maggot),
            "cg" => Ok(Self::Beaver),
            "cs" => Ok(Self::Smile),
            "ch" => Ok(Self::Eye),
            "cm" => Ok(Self::Mimic),
            "co" => Ok(Self::Octahedron),
            "hu" => Ok(Self::FamishedMan),
            "ad" => Ok(Self::AddStatue),
            "cf" => Ok(Self::CifStatue),
            "be" => Ok(Self::BeeStatue),
            "tn" => Ok(Self::TanStatue),
            "lv" => Ok(Self::LevStatue),
            "mo" => Ok(Self::MonStatue),
            "eu" => Ok(Self::EusStatue),
            "go" => Ok(Self::GorStatue),
            "jb" => Ok(Self::Jukebox),
            "eg" => Ok(Self::Egg),
            "ho" => Ok(Self::FakeEgg),
            "mm" => Ok(Self::MemoryCrystal),
            "se" => Ok(Self::SecretExit),
            "ct" => Ok(Self::Spider),
            "sd" => Ok(Self::Scaredeer),
            "cv" => Ok(Self::OrbThing),
            other => Err(Error::InvalidObject(other.to_string())),
        }
    }
}

impl FromStr for ParsedObjects {
    type Err = Error;
    /// Attempts to parse the input as a valid sequence of objects.
    ///
    /// # Errors
    ///
    /// Returns an error if any object was invalid. This includes an invalid
    /// [`ObjectId`], an invalid [`ObjectType`], an invalid [`Multiplier`],
    /// or some other invalid input.
    fn from_str(input: &str) -> Result<Self> {
        parse_objects(input)
    }
}

impl Default for Sigil {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Sigil {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::Right),
            "1" => Ok(Self::Up),
            "2" => Ok(Self::Left),
            "3" => Ok(Self::Down),
            other => Err(Error::InvalidObject(format!("bad direction {other}"))),
        }
    }
}

impl FromStr for InputValue {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "leech_count" => Self::LeechCount,
            "maggot_count" => Self::MaggotCount,
            "beaver_count" => Self::BeaverCount,
            "smile_count" => Self::SmileCount,
            "eye_count" => Self::EyeCount,
            "mimic_count" => Self::MimicCount,
            "octahedron_count" => Self::OctahedronCount,
            "spider_count" => Self::SpiderCount,
            "orb_count" => Self::OrbCount,
            "scaredeer_count" => Self::ScaredeerCount,
            "player_x" => Self::PlayerX,
            "player_y" => Self::PlayerY,
            "editor_time" => Self::EditorTime,
            "add_count" => Self::AddCount,
            "mon_count" => Self::MonCount,
            "tan_count" => Self::TanCount,
            "lev_count" => Self::LevCount,
            "eus_count" => Self::EusCount,
            "bee_count" => Self::BeeCount,
            "gor_count" => Self::GorCount,
            "cif_count" => Self::CifCount,
            "jukebox_count" => Self::JukeboxCount,
            "egg_count" => Self::EggCount,
            other => Self::Unknown(other.to_string()),
        })
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let id = &self.id;
        let tile_type = self
            .tile_type
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        let multiplier = self
            .multiplier
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        write!(f, "{id}{tile_type}{multiplier}")
    }
}

impl Display for TileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let id = match self {
            Self::Pit => "pt",
            Self::Floor => "fl",
            Self::Glass => "gl",
            Self::Bomb => "mn",
            Self::LitBomb => "xp",
            Self::FloorSwitch => "fs",
            Self::CopyFloor => "cr",
            Self::Exit => "ex",
            Self::DeathFloor => "df",
            Self::BlackFloor => "bl",
            Self::BlankFloor => "wh",
            Self::Wall => "wa",
            Self::FunhouseWall => "mw",
            Self::DISWall => "dw",
            Self::EXWall => "ew",
            Self::Edge => "ed",
            Self::DISEdge => "de",
            Self::SmallChest => "st",
        };
        write!(f, "{id}")
    }
}

impl Display for ParsedTiles {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let tiles = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect_vec()
            .join("");
        write!(f, "{tiles}")
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let id = &self.id;
        let object_type = self
            .object_type
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        let multiplier = self
            .multiplier
            .as_ref()
            .map_or_else(String::new, ToString::to_string);
        write!(f, "{id}{object_type}{multiplier}")
    }
}

impl Display for ObjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let id = match self {
            Self::Empty => "em",
            Self::Player => "pl",
            Self::Leech => "cl",
            Self::Maggot => "cc",
            Self::Beaver => "cg",
            Self::Smile => "cs",
            Self::Eye => "ch",
            Self::Mimic => "cm",
            Self::Octahedron => "co",
            Self::FamishedMan => "hu",
            Self::AddStatue => "ad",
            Self::CifStatue => "cf",
            Self::BeeStatue => "be",
            Self::TanStatue => "tn",
            Self::LevStatue => "lv",
            Self::MonStatue => "mo",
            Self::EusStatue => "eu",
            Self::GorStatue => "go",
            Self::Jukebox => "jb",
            Self::Egg => "eg",
            Self::FakeEgg => "ho",
            Self::MemoryCrystal => "mm",
            Self::SecretExit => "se",
            Self::Spider => "ct",
            Self::Scaredeer => "sd",
            Self::OrbThing => "cv",
        };
        write!(f, "{id}")
    }
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let object_type = match self {
            Self::Direction { direction } => direction.to_string(),
            Self::AddStatue1 {
                first_input,
                destroy_value,
            } => {
                let first_input = BASE64_STANDARD.encode(first_input.to_string());
                let destroy_value = BASE64_STANDARD.encode(destroy_value.to_string());
                format!("1{first_input}!{destroy_value}!")
            }
            Self::AddStatue2 {
                first_input,
                second_input,
                destroy_value,
                branefuck,
            } => {
                let first_input = BASE64_STANDARD.encode(first_input.to_string());
                let second_input = BASE64_STANDARD.encode(second_input.to_string());
                let destroy_value = BASE64_STANDARD.encode(destroy_value.to_string());
                format!("2{first_input}!{second_input}!{destroy_value}!{branefuck}!")
            }
            Self::Egg { messages } => {
                let len = messages.len();
                let messages = messages
                    .iter()
                    .map(|m| BASE64_STANDARD.encode(m.to_string()))
                    .collect_vec()
                    .join("!");
                if messages.is_empty() {
                    "0".into()
                } else {
                    format!("{len}{messages}!")
                }
            }
        };
        write!(f, "{object_type}")
    }
}

impl Display for ParsedObjects {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let objects = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect_vec()
            .join("");
        write!(f, "{objects}")
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let direction = match self {
            Self::Right => "0",
            Self::Up => "1",
            Self::Left => "2",
            Self::Down => "3",
        };
        write!(f, "{direction}")
    }
}

impl Display for InputValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let value = match self {
            Self::Unknown(value) => value,
            Self::LeechCount => "leech_count",
            Self::MaggotCount => "maggot_count",
            Self::BeaverCount => "beaver_count",
            Self::SmileCount => "smile_count",
            Self::EyeCount => "eye_count",
            Self::MimicCount => "mimic_count",
            Self::OctahedronCount => "octahedron_count",
            Self::SpiderCount => "spider_count",
            Self::OrbCount => "orb_count",
            Self::ScaredeerCount => "scaredeer_count",
            Self::PlayerX => "player_x",
            Self::PlayerY => "player_y",
            Self::EditorTime => "editor_time",
            Self::AddCount => "add_count",
            Self::MonCount => "mon_count",
            Self::TanCount => "tan_count",
            Self::LevCount => "lev_count",
            Self::EusCount => "eus_count",
            Self::BeeCount => "bee_count",
            Self::GorCount => "gor_count",
            Self::CifCount => "cif_count",
            Self::JukeboxCount => "jukebox_count",
            Self::EggCount => "egg_count",
        };
        write!(f, "{value}")
    }
}

impl IntoResponse for Sigil {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, self.to_string()).into_response()
    }
}
