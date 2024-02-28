use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const BLACK_HOLE_FORMAT: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";

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

pub struct ParsedLevel {
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

impl std::fmt::Display for ParsedLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version: {}\nName: {}\nDescription: {}\nMusic: {}\nAuthor: {}\nBrand: {}\nBurdens: {}\nTiles: {}\nObjects{}\nUploaded: {}\nEdited: {}", self.version, self.name, self.description, self.music, self.author, self.brand, self.burdens, self.tiles, self.objects, self.uploaded, self.edited)
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

impl Level {
    pub fn to_parsed(&self) -> Result<ParsedLevel> {
        Self::parse(String::from_utf8(self.data.0.clone())?.as_str())
    }

    pub fn parse(input: &str) -> Result<ParsedLevel> {
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
            .ok_or_else(|| anyhow::anyhow!("missing"))?;
        let version = version.parse::<u8>()?;
        if version != 1 {
            return Err(anyhow::anyhow!("invalid version"));
        }
        // TODO: decide character limits
        let name = String::from_utf8(BASE64_STANDARD.decode(name)?)?;
        let description = String::from_utf8(BASE64_STANDARD.decode(description)?)?;
        let music = String::from_utf8(BASE64_STANDARD.decode(music)?)?;
        if !VALID_MUSIC.contains(&music.as_str()) {
            return Err(anyhow::anyhow!("invalid music"));
        }
        let author = String::from_utf8(BASE64_STANDARD.decode(author)?)?;
        let brand = brand.parse::<u64>()?;
        if brand > BRAND_36_BITS {
            return Err(anyhow::anyhow!("invalid brand"));
        }
        // if !uploaded.is_empty() {
        //     return Err(anyhow::anyhow!("upload date should be empty"));
        // }
        // if !edited.is_empty() {
        //     return Err(anyhow::anyhow!("last edit date should be empty"));
        // }
        let burdens = burdens.parse::<u8>()?;
        if burdens > BURDENS_4_BITS {
            return Err(anyhow::anyhow!("invalid burdens"));
        }
        // TODO: is there some way to actually validate level data?
        // if any chars are invalid
        if tiles.chars().any(|char| !BLACK_HOLE_FORMAT.contains(char)) {
            return Err(anyhow::anyhow!("invalid tiles"));
        }
        if objects
            .chars()
            .any(|char| !BLACK_HOLE_FORMAT.contains(char))
        {
            return Err(anyhow::anyhow!("invalid objects"));
        }
        Ok(ParsedLevel {
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
            12 => todo!(),
            _ => todo!(),
        }
    }

    pub fn update_edited(&mut self) {
        let without_edited = &self.data.0[0..self.data.0.len() - 8];

        // 2024-02-27
        let now = OffsetDateTime::now_utc();
        // 20240227
        let now = now.date().to_string().replace('-', "");

        self.data.0 = [without_edited, now.as_bytes()].concat();
    }
}
