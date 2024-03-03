use crate::prelude::*;
use base64::{prelude::BASE64_STANDARD, Engine};
use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ulid::Ulid;

const MAX_NAME_LEN: usize = 30;
const MAX_DESCRIPTION_LEN: usize = 256;
const MAX_AUTHOR_LEN: usize = 30;
const BRAND_36_BITS: u64 = 68_719_476_735;
const BURDENS_4_BITS: u8 = 31;
const VALID_MUSIC: [&str; 11] = [
    // ambience
    "",
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
const BLACK_HOLE_FORMAT: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";

// TODO: not pub
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Data(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Version(u8);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Name(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Description(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Music(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Author(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Brand(u64);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Burdens(u8);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Tiles(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Objects(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Uploaded(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Edited(String);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Level {
    pub data: Data,
}

pub struct Parsed {
    version: Version,
    name: Name,
    description: Description,
    music: Music,
    author: Author,
    brand: Brand,
    burdens: Burdens,
    tiles: Tiles,
    objects: Objects,
    uploaded: Uploaded,
    edited: Edited,
}

impl std::fmt::Display for Parsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version: {}\nName: {}\nDescription: {}\nMusic: {}\nAuthor: {}\nBrand: {}\nBurdens: {}\nTiles: {}\nObjects: {}\nUploaded: {}\nEdited: {}", self.version, self.name, self.description, self.music, self.author, self.brand, self.burdens, self.tiles, self.objects, self.uploaded, self.edited)
    }
}

impl Parsed {
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

    pub fn set_uploaded_from(&mut self, input: Level) -> Result<()> {
        self.uploaded = input.into_parsed()?.uploaded;
        Ok(())
    }

    pub fn into_level(self) -> Level {
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
        Level { data: Data(data) }
    }
}

impl Level {
    pub const fn new(input: String) -> Self {
        Self { data: Data(input) }
    }

    pub fn new_from_put(input: &str) -> Result<(Self, Ulid)> {
        let (input, key) = input.rsplit_once('|').ok_or(Error::InvalidStructure)?;
        Ok((
            Self {
                data: Data(input.into()),
            },
            key.parse()?,
        ))
    }

    // i don't want to shorten it
    #[allow(clippy::too_many_lines)]
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

        let version = version
            .parse::<u8>()
            .map_err(|why| Error::InvalidVersion(NumberError::NotANumber(why)))?;
        if version != 1 {
            return Err(Error::InvalidVersion(NumberError::TooBig {
                max: 1,
                found: u64::from(version),
            }));
        }

        let name = String::from_utf8(
            BASE64_STANDARD
                .decode(name)
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

        let description = String::from_utf8(
            BASE64_STANDARD
                .decode(description)
                .map_err(|why| Error::InvalidDescription(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidDescription(StringError::FromUtf8(why)))?;
        if description.len() > MAX_DESCRIPTION_LEN {
            return Err(Error::InvalidDescription(StringError::TooLong {
                max: MAX_DESCRIPTION_LEN as u64,
                found: description.len() as u64,
            }));
        }

        let music = String::from_utf8(
            BASE64_STANDARD
                .decode(music)
                .map_err(|why| Error::InvalidMusic(StringError::Base64(why)))?,
        )
        .map_err(|why| Error::InvalidMusic(StringError::FromUtf8(why)))?;
        if !VALID_MUSIC.contains(&music.as_str()) {
            return Err(Error::NotASong);
        }

        let author = String::from_utf8(
            BASE64_STANDARD
                .decode(author)
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

        let brand = brand
            .parse::<u64>()
            .map_err(|why| Error::InvalidBrand(NumberError::NotANumber(why)))?;
        if brand > BRAND_36_BITS {
            return Err(Error::InvalidBrand(NumberError::TooBig {
                max: BRAND_36_BITS,
                found: brand,
            }));
        }

        let burdens = burdens
            .parse::<u8>()
            .map_err(|why| Error::InvalidBurdens(NumberError::NotANumber(why)))?;
        if burdens > BURDENS_4_BITS {
            return Err(Error::InvalidBurdens(NumberError::TooBig {
                max: u64::from(BURDENS_4_BITS),
                found: u64::from(burdens),
            }));
        }

        // TODO: is there some way to actually validate level data?
        // if any character is not in the list of allowed characters
        if tiles.chars().any(|char| !BLACK_HOLE_FORMAT.contains(char)) {
            return Err(Error::InvalidTiles);
        }

        if objects
            .chars()
            .any(|char| !BLACK_HOLE_FORMAT.contains(char))
        {
            return Err(Error::InvalidObjects);
        }

        Ok(Parsed {
            version: Version(version),
            name: Name(name),
            description: Description(description),
            music: Music(music),
            author: Author(author),
            brand: Brand(brand),
            burdens: Burdens(burdens),
            tiles: Tiles(tiles.to_string()),
            objects: Objects(objects.to_string()),
            uploaded: Uploaded(uploaded.to_string()),
            edited: Edited(edited.to_string()),
        })
    }
}
