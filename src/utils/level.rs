//! Contains the [`Level`] struct, its [`Unvalidated`]
//! and [`Validated`] states, and related constants.

use crate::prelude::*;

use crate::utils::parser::{parse_objects, parse_tiles};
use base64::{prelude::BASE64_STANDARD, Engine};
use bitvec::order::Lsb0;
use bitvec::view::BitView;
use derive_more::Display;
use image::{ImageBuffer, ImageFormat, Rgb};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, io::Cursor, marker::PhantomData, net::IpAddr, str::FromStr};
use time::OffsetDateTime;
use tracing::warn;
use ulid::Ulid;

/// A level's name's max length.
pub const MAX_NAME_LEN: usize = 30;

/// A level's decription's max length.
pub const MAX_DESCRIPTION_LEN: usize = 256;

/// A level's author's max length.
pub const MAX_AUTHOR_LEN: usize = 30;

/// A level's author brand's highest value.
///
/// Equal to 2^36-1, 68719476735, or `68_719_476_735`.
pub const BRAND_36_BITS: u64 = 0b1111_1111_1111_1111_1111_1111_1111_1111_1111;

/// A level's burdens' highest value.
///
/// Equal to 2^4-1 or 15.
pub const BURDENS_4_BITS: u8 = 0b1111;

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
const BRANEFUCK_CHARACTERS: &str = "<>+-.[]?1234567890";

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

/// All valid tile IDs.
///
/// Every ID is encoded as 2 lowercase characters, e.g. `wa` means wall.
#[non_exhaustive]
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub enum TileId {
    /// Encoded as `pt`.
    #[display("pt")]
    Pit,
    /// Encoded as `fl`.
    #[display("fl")]
    Floor,
    /// Encoded as `gl`.
    #[display("gl")]
    Glass,
    /// Encoded as `mn`.
    #[display("mn")]
    Bomb,
    /// Encoded as `xp`.
    #[display("xp")]
    LitBomb,
    /// Encoded as `fs`.
    #[display("fs")]
    FloorSwitch,
    /// Encoded as `cr`.
    #[display("cr")]
    CopyFloor,
    /// Encoded as `ex`.
    #[display("ex")]
    Exit,
    /// Encoded as `df`.
    #[display("df")]
    DeathFloor,
    /// Encoded as `bl`.
    #[display("bl")]
    BlackFloor,
    /// Encoded as `wh`.
    #[display("wh")]
    BlankFloor,
    /// Encoded as `wa`.
    #[display("wa")]
    Wall,
    /// Encoded as `mw`.
    #[display("mw")]
    FunhouseWall,
    /// Encoded as `dw`.
    #[display("dw")]
    DISWall,
    /// Encoded as `ew`.
    #[display("ew")]
    EXWall,
    /// Encoded as `ed`.
    #[display("ed")]
    Edge,
    /// Encoded as `de`.
    #[display("de")]
    DISEdge,
    /// Encoded as `st`.
    #[display("st")]
    SmallChest,
}

/// All valid object IDs.
///
/// Every ID is encoded as 2 lowercase characters, e.g. `pl` means player.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ObjectId {
    /// Encoded as `em`.
    #[display("em")]
    Empty,
    /// Encoded as `pl`.
    #[display("pl")]
    Player,
    /// Encoded as `cl`.
    #[display("cl")]
    Leech,
    /// Encoded as `cc`.
    #[display("cc")]
    Maggot,
    /// Encoded as `cg`.
    #[display("cg")]
    Beaver,
    /// Encoded as `cs`.
    #[display("cs")]
    Smile,
    /// Encoded as `ch`.
    #[display("ch")]
    Eye,
    /// Encoded as `cm`.
    #[display("cm")]
    Mimic,
    /// Encoded as `co`.
    #[display("co")]
    Octahedron,
    /// Encoded as `hu`.
    #[display("hu")]
    FamishedMan,
    /// Encoded as `ad`.
    #[display("ad")]
    AddStatue,
    /// Encoded as `cf`.
    #[display("cf")]
    CifStatue,
    /// Encoded as `be`.
    #[display("be")]
    BeeStatue,
    /// Encoded as `tn`.
    #[display("tn")]
    TanStatue,
    /// Encoded as `lv`.
    #[display("lv")]
    LevStatue,
    /// Encoded as `mo`.
    #[display("mo")]
    MonStatue,
    /// Encoded as `eu`.
    #[display("eu")]
    EusStatue,
    /// Encoded as `go`.
    #[display("go")]
    GorStatue,
    /// Encoded as `jb`.
    #[display("jb")]
    Jukebox,
    /// Encoded as `eg`.
    #[display("eg")]
    Egg,
    /// Encoded as `ho`.
    #[display("ho")]
    FakeEgg,
    /// Encoded as `mm`.
    #[display("mm")]
    MemoryCrystal,
    /// Encoded as `se`.
    #[display("se")]
    SecretExit,
    /// Encoded as `ct`.
    #[display("ct")]
    Spider,
    /// Encoded as `sd`.
    #[display("sd")]
    Scaredeer,
    /// Encoded as `cv`.
    #[display("cv")]
    OrbThing,
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
    /// This takes in a [`InputValue`] and a [`DestroyValue`],
    /// see their documentation for details.
    AddStatue1 {
        /// See [`InputValue`].
        first_input: InputValue,
        /// See [`DestroyValue`].
        destroy_value: DestroyValue,
    },
    /// A type 2 Add statue's parameters.
    ///
    /// This takes in 2 [`InputValue`]s, a [`DestroyValue`], and a
    /// [`BranefuckProgram `] program. See their documentation for details.
    AddStatue2 {
        /// See [`InputValue`].
        first_input: InputValue,
        /// See [`InputValue`].
        second_input: InputValue,
        /// See [`DestroyValue`].
        destroy_value: DestroyValue,
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
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display("X{_0}")] // X prefix, e.g. 9 -> X9
pub struct Multiplier(pub u8);

/// An egg's message.
///
/// See [`ObjectType::Egg`] for details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Message(pub String);

/// An object's direction.
///
/// Encoded as a number between `0` and `3`. I currently don't
/// know exactly which number corresponds to which direction
/// (this isn't very important for writing a parser).
///
/// This is only used for enemies, as far as I can tell.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub enum Direction {
    // TODO: what are the actual directions?
    /// Up. Unknown encoding, temporarily `0`.
    #[display("0")]
    Up,
    /// Down. Unknown encoding, temporarily `1`.
    #[display("1")]
    Down,
    /// Left. Unknown encoding, temporarily `2`.
    #[display("2")]
    Left,
    /// Right. Unknown encoding, temporarily `3`.
    #[display("3")]
    Right,
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
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub enum InputValue {
    /// A number.
    #[display("{_0}")]
    Number(u32),
    /// Global variable, encoded as `leech_count`.
    #[display("leech_count")]
    LeechCount,
    /// Global variable, encoded as `maggot_count`.
    #[display("maggot_count")]
    MaggotCount,
    /// Global variable, encoded as `beaver_count`.
    #[display("beaver_count")]
    BeaverCount,
    /// Global variable, encoded as `smile_count`.
    #[display("smile_count")]
    SmileCount,
    /// Global variable, encoded as `eye_count`.
    #[display("eye_count")]
    EyeCount,
    /// Global variable, encoded as `mimic_count`.
    #[display("mimic_count")]
    MimicCount,
    /// Global variable, encoded as `octahedron_count`.
    #[display("octahedron_count")]
    OctahedronCount,
    /// Global variable, encoded as `spider_count`.
    #[display("spider_count")]
    SpiderCount,
    /// Global variable, encoded as `orb_count`.
    #[display("orb_count")]
    OrbCount,
    /// Global variable, encoded as `scaredeer_count`.
    #[display("scaredeer_count")]
    ScaredeerCount,
    /// Global variable, encoded as `player_x`.
    ///
    /// This is between `0` and `13`.
    #[display("player_x")]
    PlayerX,
    /// Global variable, encoded as `player_y`.
    ///
    /// This is between `0` and `8`.
    #[display("player_y")]
    PlayerY,
    /// Global variable, encoded as `editor_time`.
    #[display("editor_time")]
    EditorTime,
    /// Global variable, encoded as `add_count`.
    #[display("add_count")]
    AddCount,
    /// Global variable, encoded as `mon_count`.
    #[display("mon_count")]
    MonCount,
    /// Global variable, encoded as `tan_count`.
    #[display("tan_count")]
    TanCount,
    /// Global variable, encoded as `lev_count`.
    #[display("lev_count")]
    LevCount,
    /// Global variable, encoded as `eus_count`.
    #[display("eus_count")]
    EusCount,
    /// Global variable, encoded as `bee_count`.
    #[display("bee_count")]
    BeeCount,
    /// Global variable, encoded as `gor_count`.
    #[display("gor_count")]
    GorCount,
    /// Global variable, encoded as `cif_count`.
    #[display("cif_count")]
    CifCount,
    /// Global variable, encoded as `jukebox_count`.
    #[display("jukebox_count")]
    JukeboxCount,
    /// Global variable, encoded as `egg_count`.
    #[display("egg_count")]
    EggCount,
    /// A custom global variable.
    ///
    /// Since I don't know all of the available vanilla
    /// global variables, this variant wraps a [`String`].
    #[display("{_0}")]
    Unknown(String),
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

/// An add statue's destroy value.
///
/// Add statues may take in a
/// [Branefuck program](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// and a destroy value. When the program's output matches the set
/// destroy value, the Add statue will be destroyed.
///
/// See [Endless Void's page on Branefuck](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck) for further details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct DestroyValue(u32);

/// A (possibly invalid) Void Stranger level.
///
/// See [`Data`], [`Unvalidated`], and [`Validated`] for details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display("{data}")]
pub struct Level<State = Unvalidated> {
    /// A level's (possibly invalid) data.
    ///
    /// See [`Data`] for details.
    pub data: Data,
    /// The uploader's IP adress.
    ///
    /// Stored for logging and banning.
    pub uploader: IpAddr,
    /// The level's key.
    pub key: Key,
    /// The level's current validity state. See [`Validated`] and [`Unvalidated`].
    state: PhantomData<State>,
}

/// A level's representation in the Web UI.
#[allow(clippy::module_name_repetitions)]
pub struct IndexLevel {
    /// See [`Version`].
    pub version: Version,
    /// See [`Name`].
    pub name: Name,
    /// See [`Description`].
    pub description: Description,
    /// See [`Music`].
    pub music: Music,
    /// See [`Author`].
    pub author: Author,
    /// See [`Brand`].
    pub brand: Brand,
    /// See [`BrandImage`].
    pub brand_image: BrandImage,
    /// See [`Uploaded`].
    pub uploaded: Uploaded,
    /// See [`Edited`].
    pub edited: Edited,
    /// See [`Burdens`].
    pub burdens: Burdens,
    /// See [`Key`].
    pub key: Key,
    /// The IP address of the uploader.
    pub uploader: IpAddr,
}

/// A parsed, validated Void Stranger level.
///
/// See [`Validated`] for details on level validity.
pub struct Parsed {
    /// See [`Version`].
    pub version: Version,
    /// See [`Name`].
    pub name: Name,
    /// See [`Description`].
    pub description: Description,
    /// See [`Music`].
    pub music: Music,
    /// See [`Author`].
    pub author: Author,
    /// See [`Brand`].
    pub brand: Brand,
    /// See [`Uploaded`].
    pub uploaded: Uploaded,
    /// See [`Edited`].
    pub edited: Edited,
    /// See [`Burdens`].
    pub burdens: Burdens,
    /// See [`Tiles`].
    pub tiles: Tiles,
    /// See [`Objects`].
    pub objects: Objects,
    /// See [`Key`].
    pub key: Key,
    /// The IP address of the uploader.
    pub uploader: IpAddr,
}

/// A level's data, as sent to
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).
///
/// The format is as follows:
///
/// `1|Zm9v|YmFy|bXNjXzAwMQ==|aGV4ZmFl|2685020332|20240304|20240304|0|ptX33exptX11flX2ptX10flX2ptX10flX2ptX33|emX61plemX62`
///
/// `version|name|description|music|author|brand|uploaded|edited|burdens|tiles|objects`
///
/// Note that a POST request from
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// will omit the [`Uploaded`] and [`Edited`] fields, but keep the separators:
///
/// `1|Zm9v|YmFy|bXNjXzAwMQ==|aGV4ZmFl|2685020332|||0|ptX33exptX11flX2ptX10flX2ptX10flX2ptX33|emX61plemX62`
///
/// `version|name|description|music|author|brand|||burdens|tiles|objects`
///
/// And a PUT request will do the same, but append a separator and a
/// [ULID](https://github.com/ulid/spec) key:
///
/// `1|Zm9v|YmFy|bXNjXzAwMQ==|aGV4ZmFl|2685020332|||0|ptX33exptX11flX2ptX10flX2ptX10flX2ptX33|emX61plemX62|01HR55PKF2BYRT1210Q67M8J34`
///
/// `version|name|description|music|author|brand|||burdens|tiles|objects|key`
///
/// See [`Version`], [`Name`], [`Description`], [`Music`],
/// [`Author`], [`Brand`], [`Uploaded`], [`Edited`], [`Burdens`],
/// [`Tiles`], [`Objects`], and [`Key`] for further details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Data(String);

/// The default state of a level from POST and PUT requests.
///
/// In order to be inserted into the database, the level must first
/// be parsed (and therefore validated) via [`Level::into_parsed()`].
/// before going through [`Parsed::into_level()`].
#[derive(Debug, Clone)]
pub struct Unvalidated;

/// The required state for a level being inserted into the database.
///
/// A level must go through [`Level::into_parsed()`] and then through
/// [`Parsed::into_level()`].
///
/// A validated level has a few guarantees: It has a valid format version.
/// Name, description, and author are all valid strings and lengths.
/// Music is one of the configured allowed songs. Brand and burdens are valid
/// 36-bit and 4-bit numbers, respectively. It has an upload and last edit
/// date in `yyyymmdd` format.
///
/// However, the validity of the tiles and objects is not guaranteed. There
/// is only a simple check that every character is in the configured list
/// of allowed characters.
#[derive(Debug, Clone)]
pub struct Validated;

/// The level's format version.
///
/// At the time of writing (2024-06-25), this is 1 or 2.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Version(u8);

/// The level's name.
///
/// Encoded as [`BASE64_STANDARD`], with a minimum
/// length of 1 and a max length of [`MAX_NAME_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Name(String);

/// The level's description.
///
/// Encoded as [`BASE64_STANDARD`], with no minimum,
/// but a max length of [`MAX_NAME_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Description(String);

/// The level's choice of music.
///
/// Encoded as [`BASE64_STANDARD`], it must be one of the
/// configured allowed songs from [`VoyagerConfig`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Music(String);

/// The level's author.
///
/// Encoded as [`BASE64_STANDARD`], with a minimum
/// length of 1 and a max length of [`MAX_AUTHOR_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Author(String);

/// The level's author brand.
///
/// Brand is a 6x6 grid consisting of either white or
/// black pixels. As such, the brand is encoded as 36
/// bits, and is therefore stored as a u64 in Voyager
/// and sent to/from
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// as a decimal integer.
///
/// The pixels/bits are stored in least significant order.
/// For example, a brand with the first 4 pixels white and
/// the rest black would be represented in Voyager as a u64
/// with 60 0's followed by 4 1's in binary, or 15 in decimal.
///
/// See [`BRAND_36_BITS`] for the biggest brand possible (a
/// completely white 6x6 grid).
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Brand(u64);

/// Base64-encoded 6x6 PNG of the level author's brand.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct BrandImage(String);

/// The level's original upload date.
///
/// Encoded as `yyyymmdd`, e.g. 20240304. The timezone
/// is UTC.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Uploaded(String);

/// The level's last edit date.
///
/// Encoded as `yyyymmdd`, e.g. 20240304. The timezone
/// is UTC.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Edited(String);

/// The level's burdens.
///
/// A burden is an item that gives the player special abilities
/// in-game. There are 4 possible burdens which may all be
/// independently toggled on or off. As such, the burdens can
/// be encoded as 4 bits, and are therefore stored as a u8 in
/// Voyager and sent to/from
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// as a decimal integer.
///
/// See [`BURDENS_4_BITS`] for the biggest value possible.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(u8);

/// The level's (unparsed) tiles.
///
/// Encoded in [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)'s
/// black hole format. See [`ParsedTiles::from_str`] or
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)'s
/// documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);

/// The level's parsed tiles.
///
/// See [`Tile`] for details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTiles(pub Vec<Tile>);

/// The level's (unparsed) objects.
///
/// Encoded in [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)'s
/// black hole format. See [`ParsedObjects::from_str`] or
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)'s
/// documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

/// The level's parsed objects.
///
/// See [`Object`] for details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedObjects(pub Vec<Object>);

/// The level's private key.
///
/// Encoded as a [ULID](https://github.com/ulid/spec) key.
#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Key(Ulid);

impl FromStr for TileId {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        match input {
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
            _ => Err(Error::InvalidTiles),
        }
    }
}

impl FromStr for ObjectId {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        match input {
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
            _ => Err(Error::InvalidObjects),
        }
    }
}

impl ObjectType {
    /// Turns a [`Vec`] of [`String`]s into [`ObjectType::Egg`] containing a [`Vec`] of [`Message`].
    pub fn egg(input: Vec<String>) -> Self {
        let messages = input.into_iter().map(Message).collect();
        Self::Egg { messages }
    }

    /// Attempts to parse the input as either Add statue type.
    ///
    /// If succesful, this function will return either [`ObjectType::AddStatue1`]
    /// or [`ObjectType::AddStatue2`].
    ///
    /// The input [`Vec`] must be either 2 or 4 in length.
    /// - If 2 in length, the 1st element must be a valid [`InputValue`],
    ///   and the 2nd element a valid [`DestroyValue`].
    /// - If 4 in length, the first 2 elements must be valid [`InputValue`]s.
    ///   The 3rd must be a valid [`DestroyValue`]. The 4th and final element must
    ///   be a valid [`BranefuckProgram`].
    ///
    /// Examples of valid input:
    ///
    /// 1. (``player_x``, `7`)
    /// 2. (``leech_count``, `2`, `1`, `[->-<]>?.`)

    // nom's count function returns a Vec, which
    // isn't needed here (a slice would be fine),
    // but there's no easy, non-ugly way to make
    // map_res borrow instead of moving
    // TODO: get rid of this
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_statue(input: Vec<String>) -> Result<Self> {
        match input.len() {
            2 => Ok(Self::AddStatue1 {
                first_input: InputValue::from_str(&input[0])?,
                destroy_value: DestroyValue::from_str(&input[1])?,
            }),
            4 => Ok(Self::AddStatue2 {
                first_input: InputValue::from_str(&input[0])?,
                second_input: InputValue::from_str(&input[1])?,
                destroy_value: DestroyValue::from_str(&input[2])?,
                branefuck: BranefuckProgram::from_str(&input[3])?,
            }),
            _ => Err(Error::InvalidObjects),
        }
    }

    /// Attempts to parse the input as [`ObjectType::Direction`].
    ///
    /// See [`Direction`] for details.
    pub fn direction(input: &str) -> Result<Self> {
        Ok(Self::Direction {
            direction: Direction::from_str(input)?,
        })
    }
}

impl FromStr for Direction {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        match input {
            // TODO: what are the actual directions?
            "0" => Ok(Self::Up),
            "1" => Ok(Self::Down),
            "2" => Ok(Self::Left),
            "3" => Ok(Self::Right),
            _ => Err(Error::InvalidObjects),
        }
    }
}

impl FromStr for InputValue {
    type Err = Infallible;

    fn from_str(input: &str) -> Result<Self, Infallible> {
        match input {
            "leech_count" => Ok(Self::LeechCount),
            "maggot_count" => Ok(Self::MaggotCount),
            "beaver_count" => Ok(Self::BeaverCount),
            "smile_count" => Ok(Self::SmileCount),
            "eye_count" => Ok(Self::EyeCount),
            "mimic_count" => Ok(Self::MimicCount),
            "octahedron_count" => Ok(Self::OctahedronCount),
            "spider_count" => Ok(Self::SpiderCount),
            "orb_count" => Ok(Self::OrbCount),
            "scaredeer_count" => Ok(Self::ScaredeerCount),
            "player_x" => Ok(Self::PlayerX),
            "player_y" => Ok(Self::PlayerY),
            "editor_time" => Ok(Self::EditorTime),
            "add_count" => Ok(Self::AddCount),
            "mon_count" => Ok(Self::MonCount),
            "tan_count" => Ok(Self::TanCount),
            "lev_count" => Ok(Self::LevCount),
            "eus_count" => Ok(Self::EusCount),
            "bee_count" => Ok(Self::BeeCount),
            "gor_count" => Ok(Self::GorCount),
            "cif_count" => Ok(Self::CifCount),
            "jukebox_count" => Ok(Self::JukeboxCount),
            "egg_count" => Ok(Self::EggCount),
            _ => Ok(input
                .parse::<u32>()
                .map_or_else(|_| Self::Unknown(input.into()), Self::Number)),
        }
    }
}

impl FromStr for BranefuckProgram {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        if input.chars().all(|c| BRANEFUCK_CHARACTERS.contains(c)) {
            Ok(Self(input.into()))
        } else {
            Err(Error::InvalidObjects)
        }
    }
}

impl FromStr for DestroyValue {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self(
            input.parse::<u32>().map_err(|_| Error::InvalidObjects)?,
        ))
    }
}

impl Level<Unvalidated> {
    /// Creates a new (possibly invalid) Void Stranger level, for POST.
    ///
    /// The input is not validated at this point. Therefore,
    /// the level should be parsed (validated) using
    /// [`Self::into_parsed`] before insertion into the database.
    ///
    /// See [`Data`] for details on validity.
    pub fn new(data: String, ip: IpAddr) -> Self {
        Self {
            data: Data(data),
            uploader: ip,
            key: Key::new(),
            state: PhantomData::<Unvalidated>,
        }
    }

    /// Creates a new (possibly invalid) Void Stranger level, for PUT.
    ///
    /// The input is not validated at this point. Therefore,
    /// the level should be parsed (validated) using
    /// [`Self::into_parsed`] before insertion into the database.
    ///
    /// See [`Data`] for details on validity.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is invalid.
    pub fn new_from_put(input: &str, ip: IpAddr) -> Result<Self> {
        let (input, key) = input.rsplit_once('|').ok_or(Error::InvalidStructure)?;
        Ok(Self {
            data: Data(input.into()),
            uploader: ip,
            key: key.parse()?,
            state: PhantomData::<Unvalidated>,
        })
    }
}

impl<State> Level<State> {
    /// Parses and validates the level.
    ///
    /// # Errors
    ///
    /// Returns an error if the input had an invalid structure, contained invalid
    /// Base64, produced invalid UTF-8, or does not use one of the allowed songs.
    pub fn into_parsed(self, config: &VoyagerConfig) -> Result<Parsed> {
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
        ) = self
            .data
            .0
            .splitn(11, '|')
            .collect_tuple()
            .ok_or(Error::InvalidStructure)?;

        let version = Version::try_from(version, config)?;
        let name = name.parse()?;
        let description = description.parse()?;
        let music = Music::from_str(music, config)?;
        let author = author.parse()?;
        let brand = brand.parse()?;
        let uploaded = Uploaded(uploaded.to_string());
        let edited = Edited(edited.to_string());
        let burdens = burdens.parse()?;
        let tiles = tiles.parse()?;
        let objects = objects.parse()?;
        let key = self.key;
        let ip = self.uploader;

        Ok(Parsed {
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
            key,
            uploader: ip,
        })
    }
}

impl IndexLevel {
    /// Creates a new [`IndexLevel`] from a parsed level.
    pub fn new(input: Parsed) -> Self {
        let brand_image = BrandImage::new(&input.brand);
        Self {
            version: input.version,
            name: input.name,
            description: input.description,
            music: input.music,
            author: input.author,
            brand: input.brand,
            brand_image,
            uploaded: input.uploaded,
            edited: input.edited,
            burdens: input.burdens,
            key: input.key,
            uploader: input.uploader,
        }
    }
}

impl Parsed {
    /// Sets a parsed level's upload and last
    /// edit dates to today in `yyyymmdd` format.
    pub fn set_dates_to_now(&mut self) {
        let now = OffsetDateTime::now_utc()
            // 2024-02-27
            .date()
            .to_string()
            // 20240227
            .replace('-', "");
        self.uploaded.0.clone_from(&now);
        self.edited.0 = now;
    }

    /// Sets a parsed level's upload date from
    /// another level's upload date.
    ///
    /// This is used for PUT requests, where the
    /// old level is gotten from the database to
    /// reference the level's original upload date.
    pub fn set_uploaded_from(
        &mut self,
        input: Level<Validated>,
        config: &VoyagerConfig,
    ) -> Result<()> {
        self.uploaded = input.into_parsed(config)?.uploaded;
        Ok(())
    }

    /// Decodes a level back into a Void Stranger level.
    ///
    /// For a POST and PUT requests, this is done immediately
    /// after parsing (validating) the level to insert into
    /// the database as validated.
    pub fn into_level(self) -> Level<Validated> {
        let version = self.version.0;
        let name = BASE64_STANDARD.encode(self.name.0);
        let description = BASE64_STANDARD.encode(self.description.0);
        let music = BASE64_STANDARD.encode(self.music.0);
        let author = BASE64_STANDARD.encode(self.author.0);
        let brand = self.brand.0;
        let uploaded = self.uploaded.0;
        let edited = self.edited.0;
        let burdens = self.burdens.0;
        let tiles = self.tiles;
        let objects = self.objects;
        let data = format!("{version}|{name}|{description}|{music}|{author}|{brand}|{uploaded}|{edited}|{burdens}|{tiles}|{objects}");
        Level {
            data: Data(data),
            key: self.key,
            uploader: self.uploader,
            state: PhantomData::<Validated>,
        }
    }
}

impl Version {
    /// Parses input as an integer for a level's format version
    ///
    /// # Errors
    /// Returns an error if the input wasn't a number, was too
    /// big, or was too small.
    fn try_from(input: &str, config: &VoyagerConfig) -> Result<Self> {
        let version = input
            .parse::<u8>()
            .map_err(|why| Error::InvalidVersion(NumberError::NotANumber(why)))?;
        let too_big = version > config.format_version.0;
        let is_zero = version == 0;

        if too_big {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: u64::from(config.format_version.0),
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
    /// Returns an error if the input was invalid Base64,
    /// produced invalid UTF-8, or is not one of the allowed songs.
    fn from_str(input: &str, config: &VoyagerConfig) -> Result<Self> {
        let music = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidMusic(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidMusic(StringError::FromUtf8(why)))?;
        if !config.allowed_songs.contains(&music) {
            return Err(Error::NotASong);
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
        let mut buf = Cursor::new(Vec::new());
        if let Err(why) = img.write_to(&mut buf, ImageFormat::Png) {
            warn!("something went wrong while writing image to buffer! {why}");
        };
        let png = BASE64_STANDARD.encode(buf.into_inner());
        Self(png)
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
        if let Err(why) = ParsedObjects::from_str(input) {
            warn!("error while parsing tiles: {why}");
        };
        Ok(Self(input.to_owned()))
    }
}

impl FromStr for ParsedTiles {
    type Err = Error;

    /// Attempts to parse the input as a valid sequence of tiles.
    ///
    /// # Errors
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
        };
        Ok(Self(input.to_string()))
    }
}

impl FromStr for ParsedObjects {
    type Err = Error;
    /// Attempts to parse the input as a valid sequence of objects.
    ///
    /// # Errors
    /// Returns an error if any object was invalid. This includes an invalid
    /// [`ObjectId`], an invalid [`ObjectType`], an invalid [`Multiplier`],
    /// or some other invalid input.
    fn from_str(input: &str) -> Result<Self> {
        parse_objects(input)
    }
}

impl Key {
    /// Generates a new [ULID](https://github.com/ulid/spec)
    /// key for a level.
    fn new() -> Self {
        Self(Ulid::new())
    }
}

impl FromStr for Key {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl Display for ParsedTiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tiles = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect_vec()
            .join("");
        write!(f, "{tiles}")
    }
}

impl Display for ParsedObjects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let objects = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect_vec()
            .join("");
        write!(f, "{objects}")
    }
}

impl Display for Parsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version: {}\nName: {}\nDescription: {}\nMusic: {}\nAuthor: {}\nBrand: {}\nBurdens: {}\nTiles: {}\nObjects: {}\nUploaded: {}\nEdited: {}", self.version, self.name, self.description, self.music, self.author, self.brand, self.burdens, self.tiles, self.objects, self.uploaded, self.edited)
    }
}
