//! The Void Codex's comprehensive error type.
//!
//! The variants correspond to different level requirements, e.g.
//! an invalid song, too long of a name, or invalid Base64-encoding.

use thiserror::Error;

// for documentation
#[allow(unused_imports)]
use crate::{
    BRAND_36_BITS, BURDENS_5_BITS, Direction, MAX_AUTHOR_LEN, MAX_DESCRIPTION_LEN,
    MAX_NAME_LEN, Objects, Tiles, parser,
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
        min: u64,
        /// The input.
        found: u64,
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
