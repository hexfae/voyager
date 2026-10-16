/// Classes and parsers shared by both packs and levels
use crate::prelude::*;
use bitvec::order::Lsb0;
use derive_more::Display;
use serde::{Deserialize, Serialize};

use image::imageops::{FilterType, resize};
use image::{ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;
use std::str::FromStr;
use tracing::warn;

use crate::error::{Error, NumberError, StringError};
use base64::{Engine, prelude::BASE64_STANDARD};
use bitvec::view::BitView;

use ulid::Ulid;

/// A brand's highest value.
///
/// Equal to 2^36-1, 68719476735, or `68_719_476_735`.
pub const BRAND_36_BITS: u64 = 0b1111_1111_1111_1111_1111_1111_1111_1111_1111;

/// An author's max length.
pub const MAX_AUTHOR_LEN: usize = 30;

/// A decription's max length.
pub const MAX_DESCRIPTION_LEN: usize = 256;

/// A name's max length.
pub const MAX_NAME_LEN: usize = 30;

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

#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Sigil(pub Ulid);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Cipher(pub String);

#[derive(Debug, Display, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Name(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Description(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Author(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Brand(pub u64);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Uploaded(pub String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Edited(pub String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Music(String);

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

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display("{base64}")]
pub struct BrandImage {
    pub base64: String,
    pub bytes: Vec<u8>,
}

impl Sigil {
    /// Generates a new [ULID](https://github.com/ulid/spec)
    /// key for a level.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
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

impl Author {
    pub fn from_str(input: &str, from_pack: bool) -> Result<Self> {
        let author = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidAuthor(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidAuthor(StringError::FromUtf8(why)))?;

        if !from_pack {
            if author.is_empty() {
                return Err(Error::InvalidName(StringError::TooShort));
            }
            if author.len() > MAX_AUTHOR_LEN {
                return Err(Error::InvalidName(StringError::TooLong {
                    max: MAX_AUTHOR_LEN as u64,
                    found: author.len() as u64,
                }));
            }
        }

        Ok(Self(author))
    }
}

impl Music {
    /// Parses input as music from Void Stranger.
    ///
    /// # Errors
    ///
    /// Returns an error if the input was invalid Base64,
    /// produced invalid UTF-8, or is not one of the allowed songs.
    pub fn from_str(input: &str, allowed_songs: impl AsRef<[String]>) -> Result<Self> {
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

pub fn is_branefuck_valid(branefuck: &str) -> bool {
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
