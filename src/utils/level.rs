use crate::prelude::*;
use base64::{prelude::BASE64_STANDARD, Engine};
use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const BLACK_HOLE_FORMAT: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";
const BRAND_36_BITS: u64 = 68_719_476_735;
const BURDENS_4_BITS: u8 = 31;
const VALID_MUSIC: [&str; 11] = [
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

// TODO: not pub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data(Vec<u8>);
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
struct Uploaded(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
struct Edited(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}
impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            String::from_utf8(self.0.clone()).expect("valid level data")
        )
    }
}

impl Level {
    pub fn parse(input: &str) -> Result<Parsed> {
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
        ) = input
            .splitn(11, '|')
            .collect_tuple()
            .ok_or(Error::InvalidStructure)?;
        let version = version.parse::<u8>().map_err(|_| Error::InvalidVersion)?;
        if version != 1 {
            return Err(Error::InvalidVersion);
        }
        // TODO: decide character limits
        let name = String::from_utf8(
            BASE64_STANDARD
                .decode(name)
                .map_err(|_| Error::InvalidName)?,
        )
        .map_err(|_| Error::InvalidName)?;
        let description = String::from_utf8(
            BASE64_STANDARD
                .decode(description)
                .map_err(|_| Error::InvalidDescription)?,
        )
        .map_err(|_| Error::InvalidDescription)?;
        let music = String::from_utf8(
            BASE64_STANDARD
                .decode(music)
                .map_err(|_| Error::InvalidMusic)?,
        )
        .map_err(|_| Error::InvalidMusic)?;
        if !VALID_MUSIC.contains(&music.as_str()) {
            return Err(Error::InvalidMusic);
        }
        let author = String::from_utf8(
            BASE64_STANDARD
                .decode(author)
                .map_err(|_| Error::InvalidAuthor)?,
        )
        .map_err(|_| Error::InvalidAuthor)?;
        let brand = brand.parse::<u64>().map_err(|_| Error::InvalidBrand)?;
        if brand > BRAND_36_BITS {
            return Err(Error::InvalidBrand);
        }
        let burdens = burdens.parse::<u8>().map_err(|_| Error::InvalidBurdens)?;
        if burdens > BURDENS_4_BITS {
            return Err(Error::InvalidBurdens);
        }
        // TODO: is there some way to actually validate level data?
        // if any chars are invalid
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

    fn from_post(input: &str) -> Result<Self> {
        Self::parse(input)?;
        let now = OffsetDateTime::now_utc()
            // 2024-02-27
            .date()
            .to_string()
            // 20240227
            .replace('-', "");
        let dates = format!("|{now}|{now}|");
        let input = input.replace("|||", &dates);
        Ok(Self {
            data: Data(input.as_bytes().to_vec()),
        })
    }

    pub fn from(input: &str) -> Result<Self> {
        let number_of_fields = input.split('|').collect_vec().len();
        match number_of_fields {
            11 => Self::from_post(input),
            _ => Err(Error::InvalidStructure),
        }
    }

    pub fn update_edited(&mut self, old_level: Self) -> Result<()> {
        let string = String::from_utf8(old_level.data.0).map_err(Error::InvalidLevel)?;
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
        ) = string
            .splitn(11, '|')
            .collect_tuple()
            .ok_or(Error::InternalServerError)?;

        // 2024-02-27
        let now = OffsetDateTime::now_utc();
        // 20240227
        let now = now.date().to_string().replace('-', "");

        if edited == now {
            return Ok(());
        }

        tracing::info!("{self}");

        let new_data = format!("{version}|{name}|{description}|{music}|{author}|{brand}|{uploaded}|{now}|{burdens}|{tiles}|{objects}|");
        self.data.0 = new_data.into_bytes();

        tracing::info!("{self}");

        Ok(())
    }
}
