//! Contains the [`Level`] struct, its [`Unvalidated`]
//! and [`Validated`] states, and related constants.

use crate::prelude::*;
use base64::{prelude::BASE64_STANDARD, Engine};
use bitvec::order::Lsb0;
use bitvec::view::BitView;
use derive_more::Display;
use image::{ImageBuffer, ImageFormat, Rgb};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{io::Cursor, marker::PhantomData, net::IpAddr, str::FromStr};
use time::OffsetDateTime;
use tracing::warn;
use ulid::Ulid;

#[allow(unused_imports)]
use crate::utils::server::DEFAULT_ALLOWED_CHARACTERS;

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

/// A level's data, as sent to Endless Void.
///
/// The format is as follows:
///
/// `1|Zm9v|YmFy|bXNjXzAwMQ==|aGV4ZmFl|2685020332|20240304|20240304|0|ptX33exptX11flX2ptX10flX2ptX10flX2ptX33|emX61plemX62`
///
/// `version|name|description|music|author|brand|uploaded|edited|burdens|tiles|objects`
///
/// Note that a POST request from Endless Void will omit the
/// Uploaded and Edited fields, but keep the separators:
///
/// `1|Zm9v|YmFy|bXNjXzAwMQ==|aGV4ZmFl|2685020332|||0|ptX33exptX11flX2ptX10flX2ptX10flX2ptX33|emX61plemX62`
///
/// `version|name|description|music|author|brand|||burdens|tiles|objects`
///
/// And a PUT request will do the same, but append a separator and a ULID key:
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

/// A (possibly invalid) Void Stranger level.
///
/// See [`Data`] for details.
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

/// The level's format version.
///
/// At the time of writing (2024-06-02), this is 1 or 2.
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
/// and sent to/from Endless Void as a base 10 integer.
///
/// The pixels/bits are stored in least significant order.
/// For example, a brand with the first 4 pixels white and
/// the rest black would be represented in Voyager as a u64
/// with 60 0's followed by 4 1's in binary, or 15 in base 10.
///
/// See [`BRAND_36_BITS`] for the biggest brand possible (a
/// completely white 6x6 grid).
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Brand(u64);

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
/// Voyager and sent to/from Endless Void as a base-10 integer.
///
/// See `[BURDENS_4_BITS]` for the biggest value possible.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(u8);

/// The level's tiles.
///
/// Encoded in Endless Void's black hole format. See
/// [`VoyagerConfig`] or [`DEFAULT_ALLOWED_CHARACTERS`]
/// for a list of default allowed characters.
///
/// Check Endless Void's documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);

/// The level's objects.
///
/// Encoded in Endless Void's black hole format. See
/// [`VoyagerConfig`] or [`DEFAULT_ALLOWED_CHARACTERS`]
/// for a list of default allowed characters.
///
/// Check Endless Void's documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

/// The level's private key.
///
/// Encoded as a [ULID](https://github.com/ulid/spec) key.
#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Key(Ulid);

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

#[allow(clippy::doc_markdown)]
/// A level's representation in the WebUI.
// todo: documentation
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

/// Base64-encoded 6x6 PNG of the level author's brand.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct BrandImage(String);

#[allow(clippy::doc_markdown)]
/// A level's representation in the WebUI.
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
        let name = Name::try_from(name)?;
        let description = Description::try_from(description)?;
        let music = Music::try_from(music, config)?;
        let author = Author::try_from(author)?;
        let brand = Brand::try_from(brand)?;
        let uploaded = Uploaded(uploaded.to_string());
        let edited = Edited(edited.to_string());
        let burdens = Burdens::try_from(burdens)?;
        let tiles = Tiles::try_from(tiles, config)?;
        let objects = Objects::try_from(objects, config)?;
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
        let tiles = self.tiles.0;
        let objects = self.objects.0;
        let data = format!("{version}|{name}|{description}|{music}|{author}|{brand}|{uploaded}|{edited}|{burdens}|{tiles}|{objects}");
        Level {
            data: Data(data),
            key: self.key,
            uploader: self.uploader,
            state: PhantomData::<Validated>,
        }
    }
}

impl Key {
    /// Generates a new ULID key for a level.
    fn new() -> Self {
        Self(Ulid::new())
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
        let too_big = version > config.format_version;
        let is_zero = version == 0;

        if too_big {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: u64::from(config.format_version),
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

impl TryFrom<&str> for Name {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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

impl TryFrom<&str> for Description {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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
    fn try_from(input: &str, config: &VoyagerConfig) -> Result<Self> {
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

impl TryFrom<&str> for Author {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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

impl TryFrom<&str> for Brand {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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

impl TryFrom<&str> for Burdens {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
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

impl Tiles {
    /// Parses input as tiles from Void Stranger.
    ///
    /// # Errors
    /// Returns an error if any character was not found
    /// in the config list of allowed characters.
    fn try_from(input: &str, config: &VoyagerConfig) -> Result<Self> {
        // TODO: is there some way to actually validate level data?
        // if any character is not in the list of allowed characters
        if input
            .chars()
            .any(|char| !config.allowed_characters.contains(char))
        {
            return Err(Error::InvalidTiles);
        }
        Ok(Self(input.to_string()))
    }
}

impl Objects {
    /// Parses input as objects from Void Stranger.
    ///
    /// # Errors
    /// Returns an error if any character was not found
    /// in the config list of allowed characters.
    fn try_from(input: &str, config: &VoyagerConfig) -> Result<Self> {
        // TODO: is there some way to actually validate level data?
        // if any character is not in the list of allowed characters
        if input
            .chars()
            .any(|char| !config.allowed_characters.contains(char))
        {
            return Err(Error::InvalidObjects);
        }
        Ok(Self(input.to_string()))
    }
}

impl Key {
    /// Parses input as a ULID key.
    pub fn parse(input: &str) -> Option<Self> {
        Self::from_str(input).ok()
    }
}

impl FromStr for Key {
    type Err = Error;

    fn from_str(input: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(Self(input.parse()?))
    }
}

impl std::fmt::Display for Parsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version: {}\nName: {}\nDescription: {}\nMusic: {}\nAuthor: {}\nBrand: {}\nBurdens: {}\nTiles: {}\nObjects: {}\nUploaded: {}\nEdited: {}", self.version, self.name, self.description, self.music, self.author, self.brand, self.burdens, self.tiles, self.objects, self.uploaded, self.edited)
    }
}
