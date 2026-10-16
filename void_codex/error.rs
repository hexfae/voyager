//! The Void Codex's comprehensive error type.
//!
//! The variants correspond to different level requirements, e.g.
//! an invalid song, too long of a name, or invalid Base64-encoding.

use thiserror::Error;

// for documentation
#[allow(unused_imports)]
use crate::{
    BRAND_36_BITS, BURDENS_5_BITS, Direction, MAX_AUTHOR_LEN, MAX_COMMENT_LEN, MAX_DESCRIPTION_LEN,
    MAX_NAME_LEN, MAX_PALETTE, Objects, Tiles, parser,
};
#[allow(unused_imports)]
use std::str::FromStr;

/// All level errors.
#[derive(Error, Debug)]
pub enum Error {
    /// The number of separators was invalid. Input data should contain 13
    /// separators ('|') for level uploads and 14 separators for level edits.
    #[error("invalid request structure")]
    InvalidStructure,
    /// The format version is not a number, is too small (<1), or is too big.
    ///
    /// At the time of writing (2025-06-08), the latest format version is 3.
    #[error("invalid format version: {0}")]
    InvalidVersion(NumberError),
    /// The name is too short (0), is too long (>[`MAX_NAME_LEN`]), is invalid
    /// Base64, or is decoded into invalid UTF-8.
    #[error("invalid name: {0}")]
    InvalidName(StringError),
    /// The description is too long (>[`MAX_DESCRIPTION_LEN`]), is invalid
    /// Base64, or is decoded into invalid UTF-8.
    #[error("invalid description: {0}")]
    InvalidDescription(StringError),
    /// The music is invalid Base64, or is decoded into invalid UTF-8.
    #[error("invalid music: {0}")]
    InvalidMusic(StringError),
    /// The music is not one of the allowed songs.
    #[error("invalid music: not a valid song")]
    UnknownMusic,
    /// The author is too small (0), is too big (>[`MAX_AUTHOR_LEN`]), is
    /// invalid Base64, or is decoded into invalid UTF-8.
    #[error("invalid author: {0}")]
    InvalidAuthor(StringError),
    /// The author's brand is not encoded as a number, or is too big ([`BRAND_36_BITS`]).
    #[error("invalid brand: {0}")]
    InvalidBrand(NumberError),
    /// The level's burdens are not encoded as a number, or is too big ([`BURDENS_5_BITS`]).
    #[error("invalid burdens: {0}")]
    InvalidBurdens(NumberError),
    /// The level's tiles are considered invalid by the [`parser`].
    #[error("invalid tile: {0}")]
    InvalidTile(String),
    /// The level's objects are considered invalid by the [`parser`].
    #[error("invalid object: {0}")]
    InvalidObject(String),
    /// The level's theme is not encoded as a number, or is too big ([`MAX_THEME`]).
    #[error("invalid theme: {0}")]
    InvalidTheme(NumberError),
    /// The level's bount is not encoded as a number, or is too big ([`MAX_BOUNT`]), or if it's too small ([`MIN_BOUNT`])
    #[error("invalid bount: {0}")]
    InvalidBount(NumberError),
    /// The key is not a valid [ULID](https://github.com/ulid/spec) key.
    #[error("key error: {0}")]
    InvalidKey(#[from] ulid::DecodeError),
    /// An infallible (impossible) error.
    ///
    /// This is used in the [`FromStr`] implementations for
    /// [`Tiles`] and [`Objects`], since implementing [`FromStr`]
    /// is apparently preferred to [`From<&str>`]
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    /// One of the node's exit is not encoded as a number
    #[error("invalid exit: {0}")]
    InvalidExit(NumberError),
    /// One of the node's position is not encoded as two numbers seperated by a comma
    #[error("invalid position")]
    InvalidPosition,
    /// One of the node's position's coordinate is not encoded as signed number
    #[error("invalid position coordinate: {0}")]
    InvalidPositionCoordinate(NumberError),
    /// One of the pack's node is invalid
    #[error("invalid node: {0}")]
    InvalidNode(String),
    /// The root node fall value is not 0 or 1
    #[error("invalid fall: {0}")]
    InvalidFall(String),
    /// The comment node is too long (>[`MAX_COMMENT_LEN`]), is invalid
    /// Base64, or is decoded into invalid UTF-8.
    #[error("invalid comment: {0}")]
    InvalidComment(StringError),
    /// The node's palette is not encoded as a number, or is too big (>[`MAX_PALETTE`]).
    #[error("invalid palette: {0}")]
    InvalidPalette(NumberError),
    /// The pack's password is invalid base64, or is decoded into invalid UTF-8.
    #[error("invalid password: {0}")]
    InvalidPassword(StringError),
}

/// All number-related level errors.
#[derive(Error, Debug)]
// i don't want an enum with the name "Number" lol
#[allow(clippy::module_name_repetitions)]
pub enum NumberError {
    /// The input is not a number.
    #[error("not a number: {0}")]
    NotANumber(#[from] std::num::ParseIntError),
    /// The input is too small of a number.
    #[error("too small of a number: {found} < {min}")]
    TooSmall {
        /// The lowest allowed number.
        min: i64,
        /// The input.
        found: i64,
    },
    /// The input is too big of a number.
    #[error("too big of a number: {found} > {max}")]
    TooBig {
        /// The highest allowed number.
        max: u64,
        /// The input.
        found: u64,
    },
}

/// All string-related level errors.
#[derive(Error, Debug)]
// i don't want an enum with the name "String" lol
#[allow(clippy::module_name_repetitions)]
pub enum StringError {
    /// The input is invalid Base64.
    #[error("invalid base64")]
    Base64(#[from] base64::DecodeError),
    /// The input is decoded into invalid UTF-8.
    #[error("utf8 error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// The input is too short.
    #[error("input was too short: 0 < 1")]
    TooShort,
    /// The input is too long.
    #[error("input was too long: {found} > {max}")]
    TooLong {
        /// The longest allowed input.
        ///
        /// See [`MAX_NAME_LEN`], [`MAX_AUTHOR_LEN`],
        /// and [`MAX_DESCRIPTION_LEN`] for details.
        max: u64,
        /// The input.
        found: u64,
    },
}

/// All monogram-related errors.
#[derive(Error, Debug)]
pub enum MonogramError {
    /// one of `hint_type`, `width`, `height`, `target_col` or `target_row`
    /// is missing from the monogram
    #[error("missing property")]
    MissingProperty,

    // monogram's width or height is invalid
    #[error("invalid range: {found} not in [{min}, {max}]")]
    InvalidRange { min: u8, max: u8, found: u8 },

    // number of tiles is not exactly width * height
    #[error("invalid number of tiles: {found} != {expected} ")]
    InvalidNumberOfTiles { expected: usize, found: usize },
}
