//! Contains the `Level` and `ParsedLevel` structs, related
//! constants, and related wrapper types for `ParsedLevel`.

use std::marker::PhantomData;

use crate::prelude::*;
use base64::{prelude::BASE64_STANDARD, Engine};
use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
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

/// All available music choices in Void Stranger.
///
/// NOTE: An empty string (`""`) is allowed and means ambience.
pub const VALID_MUSIC: [&str; 11] = [
    "", // ambience
    "msc_001",
    "msc_dungeon_wings",
    "msc_beecircle",
    "msc_dungeongroove",
    "msc_013",
    "msc_gorcircle_lo",
    "msc_levcircle",
    "msc_cifcircle",
    "msc_beesong",
    "msc_monstrail",
];

/// All possible characters from Endless Void's black hole format.
///
/// Currently, there is no (easy) way to check if a level is valid.
/// Therefore, this is the best (easiest) way to check a level's validity.
pub const BLACK_HOLE_FORMAT: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";

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

/// The default state of a level from POST and PUT requests. In
/// order to be inserted into the database, the level must first
/// be parsed (and therefore validated) via [`Level::into_parsed()`].
/// before going through [`Parsed::into_level()`].
#[derive(Debug, Clone)]
pub struct Unvalidated;

/// The required state for a level being inserted into the database. A
/// level must go through [`Level::into_parsed()`] and then through
/// [`Parsed::into_level()`].
///
/// A validated level has a few guarantees: It has a valid version format.
/// Name, description, and author are all valid strings and lengths.
/// Music is one of eleven [`VALID_MUSIC`]. Brand and burdens are valid
/// 36-bit and 4-bit numbers, respectively. It has an upload and last edit
/// date in `yyyymmdd` format.
///
/// The only thing that is not guaranteed is the validity of the tiles and
/// objects. There is only a simple check that no character is invalid
/// (Endless Void would never generate it) according to [`BLACK_HOLE_FORMAT`].
#[derive(Debug, Clone)]
pub struct Validated;

/// A (possibly invalid) Void Stranger level.
///
/// See [`Data`] for details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[display(fmt = "{data}")]
pub struct Level<State = Unvalidated> {
    /// A level's (possibly invalid) data.
    ///
    /// See [`Data`] for details.
    pub data: Data,
    /// The level's current validity state. See [`Validated`] and [`Unvalidated`].
    state: PhantomData<State>,
}

/// The level format version.
///
/// Currently, this is only 1.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Version(u8);

/// The level's name.
///
/// Encoded as standard Base64, with a
/// minimum length of 1 and a max length of [`MAX_NAME_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Name(String);

/// The level's description.
///
/// Encoded as standard Base64,
/// with no minimum, but a max length of [`MAX_NAME_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Description(String);

/// The level's choice of music.
///
/// Encoded as standard
/// Base64, it must be one of [`VALID_MUSIC`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Music(String);

/// The level's author.
///
/// Encoded as standard Base64, with a
/// minimum length of 1 and a max length of [`MAX_AUTHOR_LEN`].
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Author(String);

/// The level's author brand.
///
/// Brand is a 6x6 grid consisting
/// of either white or black pixels. As such, the brand is
/// encoded as 36 bits, and is therefore stored as a u64 in
/// Voyager and sent to/from Endless Void as a base-10 integer.
///
/// See `BRAND_36_BITS` for the biggest brand possible.
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
/// There are 4 possible burdens that may be on or off.
/// As such, the burdens can be encoded as 4 bits, and
/// is therefore stored as a u8 in Voyager and sent to/from
/// Endless Void as a base-10 integer.
///
/// See `BURDENS_4_BITS` for the biggest value possible.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(u8);

/// The level's tiles.
///
/// Encoded in Endless Void's black hole format. See
/// [`BLACK_HOLE_FORMAT`] for all allowed characters.
/// Check Endless Void's documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);

/// The level's objects.
///
/// Encoded in Endless Void's black hole format. See
/// [`BLACK_HOLE_FORMAT`] for all allowed characters.
/// Check Endless Void's documentation for more details.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

/// The level's private key.
///
/// Encoded as a [ULID](https://github.com/ulid/spec) key.
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Key(pub Ulid);

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
    ///
    /// Currently, this is always set to 0
    /// unless a level is parsed for the
    /// Web UI. This is a really bad and
    /// non-idiomatic way to do this, but
    /// it was the simplest for now. TODO
    pub key: Key,
}

impl Level<Unvalidated> {
    /// Creates a new (possibly invalid) Void Stranger level, for POST.
    ///
    /// See [`Data`] for details on valid POST input.
    ///
    /// The input is not validated at this point. Therefore,
    /// the level should be parsed (validated) using
    /// [`Self::into_parsed`] before insertion into the database.
    pub const fn new(input: String) -> Self {
        Self {
            data: Data(input),
            state: PhantomData::<Unvalidated>,
        }
    }

    /// Creates a new (possibly invalid) Void Stranger level, for PUT.
    ///
    /// Returns a level and its (possible) key.
    ///
    /// See [`Data`] for details on valid PUT input.
    ///
    /// The input is not validated at this point. Therefore,
    /// the level should be parsed (validated) using
    /// [`Self::into_parsed`] before insertion into the database.
    /// See [`Data`] for details on validity.
    pub fn new_from_put(input: &str) -> Result<(Self, Ulid)> {
        let (input, key) = input.rsplit_once('|').ok_or(Error::InvalidStructure)?;
        Ok((
            Self {
                data: Data(input.into()),
                state: PhantomData::<Unvalidated>,
            },
            key.parse()?,
        ))
    }
}

impl<State> Level<State> {
    /// Parses and validates the level, returning
    /// an appropriate [`Error`] if unsuccessful.
    pub fn into_parsed(self) -> Result<Parsed> {
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

        let version = Version::try_from(version)?;
        let name = Name::try_from(name)?;
        let description = Description::try_from(description)?;
        let music = Music::try_from(music)?;
        let author = Author::try_from(author)?;
        let brand = Brand::try_from(brand)?;
        let uploaded = Uploaded(uploaded.to_string());
        let edited = Edited(edited.to_string());
        let burdens = Burdens::try_from(burdens)?;
        let tiles = Tiles::try_from(tiles)?;
        let objects = Objects::try_from(objects)?;

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
            key: Key(Ulid(0)),
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
        self.uploaded.0 = now.clone();
        self.edited.0 = now;
    }

    /// Sets a parsed level's upload date from
    /// another level's upload date.
    ///
    /// This is used for PUT requests, where the
    /// old level is gotten from the database to
    /// reference the level's original upload date.
    pub fn set_uploaded_from(&mut self, input: Level<Validated>) -> Result<()> {
        self.uploaded = input.into_parsed()?.uploaded;
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
            state: PhantomData::<Validated>,
        }
    }
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        let version = input
            .parse::<u8>()
            .map_err(|why| Error::InvalidVersion(NumberError::NotANumber(why)))?;
        if version != 1 {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: 1,
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

impl TryFrom<&str> for Music {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        let music = String::from_utf8(
            BASE64_STANDARD
                .decode(input)
                .map_err(|why| Error::InvalidMusic(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidMusic(StringError::FromUtf8(why)))?;
        if !VALID_MUSIC.contains(&music.as_str()) {
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

impl TryFrom<&str> for Tiles {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: is there some way to actually validate level data?
        // if any character is not in the list of allowed characters
        if input.chars().any(|char| !BLACK_HOLE_FORMAT.contains(char)) {
            return Err(Error::InvalidTiles);
        }
        Ok(Self(input.to_string()))
    }
}

impl TryFrom<&str> for Objects {
    type Error = Error;

    fn try_from(input: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        if input.chars().any(|char| !BLACK_HOLE_FORMAT.contains(char)) {
            return Err(Error::InvalidObjects);
        }
        Ok(Self(input.to_string()))
    }
}

impl std::fmt::Display for Parsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version: {}\nName: {}\nDescription: {}\nMusic: {}\nAuthor: {}\nBrand: {}\nBurdens: {}\nTiles: {}\nObjects: {}\nUploaded: {}\nEdited: {}", self.version, self.name, self.description, self.music, self.author, self.brand, self.burdens, self.tiles, self.objects, self.uploaded, self.edited)
    }
}
