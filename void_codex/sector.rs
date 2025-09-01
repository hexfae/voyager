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
use time::OffsetDateTime;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use std::{convert::Infallible, io::Cursor, net::IpAddr};
use tracing::warn;
use ulid::Ulid;
use unicode_segmentation::UnicodeSegmentation;

const VOID_STRANGER_LEVEL_WIDTH: usize = 14;

const SECTION_COUNT: usize = 13;

/// A level's author's brand's highest value.
///
/// Equal to 2^36-1, 68719476735, or `68_719_476_735`.
pub const BRAND_36_BITS: u64 = 0b1111_1111_1111_1111_1111_1111_1111_1111_1111;

/// All of the valid
/// [Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// characters.
///
/// These are the standard [Brainfuck](https://en.wikipedia.org/wiki/Brainfuck)
/// characters, plus a few additional characters added by Endless Void.
/// Additional, because all alphanumeric characters are allowed too.
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck) for details.
pub const ADDITIONAL_BRANEFUCK_CHARACTERS: &str = ".,+-[]><?^_#:;\n ";

/// A level's burdens' highest value.
///
/// Equal to 2^5-1 or 32.
pub const BURDENS_5_BITS: u8 = 0b11111;

/// The max theme number.
/// There's only two themes.
pub const MAX_THEME: u8 = 1;

/// The max brane count (Inclusive)
pub const MAX_BOUNT: i16 = 999;

// The minimum brane count (Inclusive)
pub const MIN_BOUNT: i16 = -1;

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
    pub theme: Theme,
    pub bount: Bount,
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
    idol: bool,
}

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTiles(Vec<Tile>);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedObjects(Vec<Object>);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Theme(u8);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Bount(i16);

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
    /// Encoded as `tr`.
    Tree,
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
    /// Encoded as `mu`.
    Mural,
    /// Encoded as `ts`.
    TisStatue,
}

/// An object's type.
///
/// There are 3 different types of object types, see
/// their respective documentation for details:
/// 1. [`Self::Direction`]
/// 2. [`Self::AddStatue`]
/// 3. [`Self::Egg`]
///
/// Note: An enemy with a direction of up (e.g. `ct0`)
/// will get its type parsed as [`ObjectType::Egg`] with 0
/// messages, instead of [`ObjectType::Direction`] with
/// [`Direction::Up`]. This is fine, however, because both
/// will be encoded as `0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    /// This object's [`Direction`]. It's not actually a direction, but this represents every object that just has a single number after it.
    Direction {
        /// See [`Direction`].
        direction: Direction,
    },

    /// An Add statue's parameters in either Simple or BRANEFUCK mode.
    /// It should be noted that, in simple mode, the branefuck program parameter would just
    /// be the name of a global variable or a number. Which isn't valid branefuck, but internally
    /// it's inserted into a premade branefuck program so it can be thought of as an excerpt (which means it can be validated the same way we validate branefuck)
    ///
    /// All types of add statues since format version 3 use this.
    AddStatue {
        mode: u8,
        branefuck: BranefuckProgram,
        destroy_value: InputValue,
    },

    /// This object's horizontal and vertical [`Offset`]
    ///
    /// The object will be moved horizontally and vertically by offset_x and offset_y.
    ///
    /// Only one object currently uses this, that being [`ObjectId::MemoryCrystal``].
    Offset { offset_x: i8, offset_y: i8 },

    /// A secret exit's parameters.
    ///
    /// Secret exits have an effect type, a horizontal offset, and a vertical offset.
    SecretExit {
        effect: u8,
        offset_x: i8,
        offset_y: i8,
    },

    /// A mural's parameters.
    ///
    /// Murals hold a brand and a message encoded inside that brand.
    Mural { brand: Brand, message: Message },

    /// A type 1 Add statue's parameters.
    ///
    /// This takes in 2 [`InputValue`]s, see its documentation for details.
    ///
    /// Unused since format version 3.
    AddStatue1 {
        /// See [`InputValue`].
        first_input: InputValue,
        /// See [`InputValue`].
        destroy_value: InputValue,
    },

    /// A type 2 Add statue's parameters.
    ///
    /// This takes in 3 [`InputValue`]s and a [`BranefuckProgram`]
    /// program. See their documentation for details.
    ///
    /// Unused since format version 3
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
    /// An egg has a list of messages that will be displayed when interacted with. The
    /// length may be (and is often) 0. The maximum length is 4.
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
/// Encoded as a number between `0` and `3`.
/// Starts at 0 meaning right, and moves counter-clockwise, as is often done in math.
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

/// A [Branefuck program](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck).
///
/// Add statues are able to run
/// [Branefuck programs](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// in [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// See [`ADDITIONAL_BRANEFUCK_CHARACTERS`] for details on valid characters.
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck) for further details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct BranefuckProgram(pub String);

/// An input value for Add statues.
/// It can be a number, or a global variable name. We don't care which one.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct InputValue(pub String);

/// An egg/mural's message.
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
        let sections: Vec<&str> = self.cipher.0.splitn(SECTION_COUNT, '|').collect();
        if sections.len() != SECTION_COUNT {
            return Err(Error::InvalidStructure);
        }
        let version = sections[0];
        let name = sections[1];
        let description = sections[2];
        let music = sections[3];
        let author = sections[4];
        let brand = sections[5];
        //let uploaded = sections[6];
        //let edited = sections[7];
        let burdens = sections[8];
        let tiles = sections[9];
        let objects = sections[10];
        let theme = sections[11];
        let bount = sections[12];

        let now = OffsetDateTime::now_utc()
            // 2025-03-02
            .date()
            .to_string()
            // 20250302
            .replace('-', "");
        self.cipher = Cipher(format!(
            "{version}|{name}|{description}|{music}|{author}|{brand}|{now}|{now}|{burdens}|{tiles}|{objects}|{theme}|{bount}"
        ));
        self.compendium.uploaded = Uploaded(now.clone());
        self.compendium.edited = Edited(now);
        Ok(())
    }
    

    pub fn set_uploaded_from(&mut self, sector: &Self) -> Result<()> {
        let sections: Vec<&str> = self.cipher.0.splitn(SECTION_COUNT, '|').collect();
        if sections.len() != SECTION_COUNT {
            return Err(Error::InvalidStructure);
        }
        let version = sections[0];
        let name = sections[1];
        let description = sections[2];
        let music = sections[3];
        let author = sections[4];
        let brand = sections[5];
        let uploaded = sector.compendium.uploaded.clone();
        let edited = sections[7];
        let burdens = sections[8];
        let tiles = sections[9];
        let objects = sections[10];
        let theme = sections[11];
        let bount = sections[12];
        self.cipher = Cipher(format!(
            "{version}|{name}|{description}|{music}|{author}|{brand}|{uploaded}|{edited}|{burdens}|{tiles}|{objects}|{theme}|{bount}"
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
    pub fn sigil(&self) -> Sigil {
        self.sigil
    }

    #[must_use]
    pub fn sigil_string(&self) -> String {
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
        let sections: Vec<&str> = cipher.as_ref().splitn(SECTION_COUNT, '|').collect();

        if sections.len() != SECTION_COUNT {
            return Err(Error::InvalidStructure);
        }

        let version = sections[0];
        let name = sections[1];
        let description = sections[2];
        let music = sections[3];
        let author = sections[4];
        let brand = sections[5];
        let uploaded = sections[6];
        let edited = sections[7];
        let burdens = sections[8];
        let tiles = sections[9];
        let objects = sections[10];
        let theme = sections[11];
        let bount = sections[12];

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

        let theme = theme.parse()?;
        let bount = bount.parse()?;

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
            theme,
            bount,
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
            | Self::GorStatue
            | Self::TisStatue => "🗿",
            Self::Jukebox => "📻️",
            Self::Egg | Self::FakeEgg => "🪨",
            Self::MemoryCrystal => "✨",
            Self::Spider => "🕷️",
            Self::Scaredeer => "🦌",
            Self::OrbThing => "💡",
            Self::Tree => "🌳",
            Self::Mural => "📜",
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
        let idol = if burdens.idol { "⚕️" } else { "⬜️" };
        let hud_tiles = format!("⬜️OD⬜️🪰0{rod}{idol}{memory}{wings}{sword}⬜️V?");
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
    pub fn add_statue((prefix, (first, second)): (char, (String, String))) -> Result<Self> {
        let prefix = u8::try_from(prefix).map_err(|why| Error::InvalidObject(why.to_string()))? 
            - '0' as u8;
        let first = BranefuckProgram::from_str(&first)
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        let second =
            InputValue::from_str(&second).map_err(|why| Error::InvalidObject(why.to_string()))?;
        Ok(Self::AddStatue {
            mode: prefix,
            branefuck: first,
            destroy_value: second,
        })
        // Ok(Self::AddStatue {
        //     mode: u8::from_str(&input[0]).map_err(|why| Error::InvalidObject(why.to_string()))?,
        //     branefuck: BranefuckProgram::from_str(&input[1])
        //         .map_err(|why| Error::InvalidObject(why.to_string()))?,
        //     destroy_value: InputValue::from_str(&input[2])
        //         .map_err(|why| Error::InvalidObject(why.to_string()))?,
        // })
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

    pub fn offset(
        ((first_sign, x), (second_sign, y)): ((Option<char>, &str), (Option<char>, &str)),
    ) -> Result<Self> {
        let mut x = x
            .parse::<i8>()
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        let mut y = y
            .parse::<i8>()
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        if first_sign.is_some() {
            x = -x;
        }
        if second_sign.is_some() {
            y = -y;
        }

        Ok(Self::Offset {
            offset_x: x,
            offset_y: y,
        })
    }

    pub fn secret_exit(
        (effect, (first_sign, x), (second_sign, y)): (
            &str,
            (Option<char>, &str),
            (Option<char>, &str),
        ),
    ) -> Result<Self> {
        let effect = effect
            .parse::<u8>()
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        let mut x = x
            .parse::<i8>()
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        let mut y = y
            .parse::<i8>()
            .map_err(|why| Error::InvalidObject(why.to_string()))?;
        if first_sign.is_some() {
            x = -x;
        }
        if second_sign.is_some() {
            y = -y;
        }

        Ok(ObjectType::SecretExit {
            effect,
            offset_x: x,
            offset_y: y,
        })
    }

    pub fn mural((brand, message): (&str, String)) -> Result<Self> {
        Ok(ObjectType::Mural {
            brand: brand.parse()?,
            message: Message(message),
        })
    }
}

fn is_branefuck_valid(branefuck: &str) -> bool {
    branefuck
        .chars()
        .all(|c| ADDITIONAL_BRANEFUCK_CHARACTERS.contains(c) || c.is_alphanumeric())
}

impl FromStr for BranefuckProgram {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        if is_branefuck_valid(input) {
            Ok(Self(input.into()))
        } else {
            Err(Error::InvalidObject(
                "invalid branefuck character found".into(),
            ))
        }
    }
}

impl FromStr for InputValue {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        if is_branefuck_valid(input) {
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
        // the parser does not support versions lower than 3
        let too_small = version < 3;

        if too_big {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: u64::from(latest_version),
                found: u64::from(version),
            }));
        }
        if too_small {
            return Err(Error::InvalidVersion(NumberError::TooSmall {
                min: 3,
                found: i64::from(version),
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
        if burdens > BURDENS_5_BITS {
            return Err(Error::InvalidBurdens(NumberError::TooBig {
                max: u64::from(BURDENS_5_BITS),
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
            idol: bits[4],
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
            "tr" => Ok(Self::Tree),
            "mu" => Ok(Self::Mural),
            "ts" => Ok(Self::TisStatue),
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

impl FromStr for Theme {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        // TODO: only allow matching versions to be uploaded
        let theme = input
            .parse::<u8>()
            .map_err(|why| Error::InvalidTheme(NumberError::NotANumber(why)))?;

        let too_big = theme > MAX_THEME;

        if too_big {
            return Err(Error::InvalidTheme(NumberError::TooBig {
                max: u64::from(MAX_THEME),
                found: u64::from(theme),
            }));
        }

        Ok(Self(theme))
    }
}

impl FromStr for Bount {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        // TODO: only allow matching versions to be uploaded
        let bount = input
            .parse::<i16>()
            .map_err(|why| Error::InvalidBount(NumberError::NotANumber(why)))?;

        let too_big = bount > MAX_BOUNT;
        let too_small = bount < MIN_BOUNT;

        if too_big {
            return Err(Error::InvalidBount(NumberError::TooBig {
                max: MAX_BOUNT as u64,
                found: bount as u64,
            }));
        }
        if too_small {
            return Err(Error::InvalidBount(NumberError::TooSmall {
                min: i64::from(MIN_BOUNT),
                found: i64::from(bount),
            }));
        }
        Ok(Self(bount))
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
            Self::Mural => "mu",
            Self::Tree => "tr",
            Self::TisStatue => "ts",
        };
        write!(f, "{id}")
    }
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let object_type = match self {
            Self::Direction { direction } => direction.to_string(),
            Self::AddStatue {
                mode,
                branefuck,
                destroy_value,
            } => {
                let branefuck = BASE64_STANDARD.encode(branefuck.to_string());
                let destroy_value = BASE64_STANDARD.encode(destroy_value.to_string());
                format!("{mode}!{branefuck}!{destroy_value}!")
            }
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
                branefuck,
                destroy_value,
            } => {
                let branefuck = BASE64_STANDARD.encode(branefuck.to_string());
                let destroy_value = BASE64_STANDARD.encode(destroy_value.to_string());
                // TODO: This may not be the correct order but this is legacy anyways
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
            Self::Offset { offset_x, offset_y } => {
                format!("{offset_x}!{offset_y}!")
            }
            Self::SecretExit {
                effect,
                offset_x,
                offset_y,
            } => {
                format!("{effect}!{offset_x}!{offset_y}!")
            }
            Self::Mural { brand, message } => {
                format!("{brand}!{message}!")
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

impl IntoResponse for Sigil {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, self.to_string()).into_response()
    }
}
