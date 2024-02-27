use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use derive_more::Display;
use nom::{
    bytes::complete::take_while1,
    character::complete::char,
    combinator::map_res,
    sequence::{preceded, tuple},
    IResult,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const BLACK_HOLE_FORMAT: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data(Vec<u8>);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Version(u8);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Name(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Description(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Music(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Author(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Brand(u64);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(u8);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Uploaded(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Edited(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub data: Data,
}

pub struct ParsedLevel {
    pub version: Version,
    pub name: Name,
    pub description: Description,
    pub music: Music,
    pub author: Author,
    pub brand: Brand,
    pub burdens: Burdens,
    pub tiles: Tiles,
    pub objects: Objects,
    pub uploaded: Uploaded,
    pub edited: Edited,
}

impl std::fmt::Display for ParsedLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Level \"{}\", by {}, level format {}\nDescription: \"{}\"\nMusic: {}\nBurdens: {}\nBrand: {}\nTiles: {}\nObject: {}", self.name, self.author, self.version, self.description, self.music, self.burdens, self.brand, self.tiles, self.objects)
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
    #[must_use]
    pub fn from(input: &str) -> Self {
        // 2024-02-27
        let now = OffsetDateTime::now_utc();
        // 20240227
        let now = now.date().to_string().replace('-', "");
        // remove last |
        let asd = &input.as_bytes()[0..input.len() - 1];
        // 20240227|20240227
        let uploaded_edited = format!("{now}|{now}");
        let data = [asd, uploaded_edited.as_bytes()].concat();
        Self { data: Data(data) }
    }

    /// # Errors
    /// Returns an error on invalid level data.
    pub fn parse(&self) -> IResult<&[u8], ParsedLevel> {
        let (input, output) = parse_level(&self.data.0)?;
        Ok((input, output))
    }
}

fn parse_level(input: &[u8]) -> IResult<&[u8], ParsedLevel> {
    let (
        input,
        (
            version,
            name,
            description,
            music,
            author,
            brand,
            burdens,
            tiles,
            objects,
            uploaded,
            edited,
        ),
    ) = tuple((
        version,
        base64,
        base64,
        base64,
        base64,
        brand,
        burdens,
        black_hole_format,
        black_hole_format,
        date,
        date,
    ))(input)?;
    let level = ParsedLevel {
        version: Version(version),
        name: Name(name),
        description: Description(description),
        music: Music(music),
        author: Author(author),
        brand: Brand(brand),
        burdens: Burdens(burdens),
        tiles: Tiles(tiles),
        objects: Objects(objects),
        uploaded: Uploaded(uploaded),
        edited: Edited(edited),
    };
    Ok((input, level))
}

fn version(input: &[u8]) -> IResult<&[u8], u8> {
    map_res(take_while1(is_digit), to_u8)(input)
}

fn base64(input: &[u8]) -> IResult<&[u8], String> {
    map_res(preceded(char('|'), take_while1(is_base64)), from_base64)(input)
}

fn brand(input: &[u8]) -> IResult<&[u8], u64> {
    map_res(preceded(char('|'), take_while1(is_digit)), to_u64)(input)
}

fn burdens(input: &[u8]) -> IResult<&[u8], u8> {
    map_res(preceded(char('|'), take_while1(is_digit)), to_u8)(input)
}

fn black_hole_format(input: &[u8]) -> IResult<&[u8], String> {
    map_res(
        preceded(char('|'), take_while1(is_black_hole_format)),
        to_string,
    )(input)
}

fn date(input: &[u8]) -> IResult<&[u8], String> {
    map_res(preceded(char('|'), take_while1(is_digit)), to_string)(input)
}

fn from_base64(input: &[u8]) -> Result<String> {
    Ok(String::from_utf8(BASE64_STANDARD.decode(input)?)?)
}

fn to_u8(input: &[u8]) -> Result<u8> {
    Ok(to_string(input)?.parse::<u8>()?)
}

fn to_u64(input: &[u8]) -> Result<u64> {
    Ok(to_string(input)?.parse::<u64>()?)
}

fn to_string(input: &[u8]) -> Result<String> {
    Ok(std::str::from_utf8(input)?.to_string())
}

const fn is_digit(chr: u8) -> bool {
    chr.is_ascii_digit()
}

fn is_base64(chr: u8) -> bool {
    BASE64.contains(&chr)
}

fn is_black_hole_format(chr: u8) -> bool {
    BLACK_HOLE_FORMAT.contains(&chr)
}
