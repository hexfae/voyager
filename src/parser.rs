use base64::{prelude::BASE64_STANDARD, Engine};
use color_eyre::Result;
use derive_more::Display;
use nom::{
    bytes::complete::take_while1,
    character::complete::char,
    combinator::map_res,
    sequence::{preceded, tuple},
    IResult,
};
use serde::{Deserialize, Serialize};

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
const BLACK_HOLE_FORMAT: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=!";

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Data(String);
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
pub struct Brand(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Burdens(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Tiles(String);
#[derive(Debug, Display, Clone, Serialize, Deserialize)]
pub struct Objects(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub data: Data,
    pub version: Version,
    pub name: Name,
    pub description: Description,
    pub music: Music,
    pub author: Author,
    pub brand: Brand,
    pub burdens: Burdens,
    pub tiles: Tiles,
    pub objects: Objects,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Level \"{}\", by {}, level format {}\nDescription: \"{}\"\nMusic: {}\nBurdens: {}\nBrand: {}\nTiles: {}\nObject: {}", self.name, self.author, self.version, self.description, self.music, self.burdens, self.brand, self.tiles, self.objects)
    }
}

impl Level {
    /// # Errors
    /// Returns an error on invalid level data.
    pub fn from(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, output) = parse_level(input)?;
        Ok((input, output))
    }
}

fn parse_level(input: &[u8]) -> IResult<&[u8], Level> {
    let data = String::from_utf8_lossy(input).to_string();
    let (input, (version, name, description, music, author, brand, burdens, tiles, objects)) =
        tuple((
            version,
            base64,
            base64,
            base64,
            base64,
            bits,
            bits,
            black_hole_format,
            black_hole_format,
        ))(input)?;
    let level = Level {
        data: Data(data),
        version: Version(version),
        name: Name(name),
        description: Description(description),
        music: Music(music),
        author: Author(author),
        brand: Brand(brand),
        burdens: Burdens(burdens),
        tiles: Tiles(tiles),
        objects: Objects(objects),
    };
    Ok((input, level))
}

fn version(input: &[u8]) -> IResult<&[u8], u8> {
    map_res(take_while1(is_digit), to_u8)(input)
}

fn base64(input: &[u8]) -> IResult<&[u8], String> {
    map_res(preceded(char('|'), take_while1(is_base64)), from_base64)(input)
}

fn bits(input: &[u8]) -> IResult<&[u8], String> {
    map_res(preceded(char('|'), take_while1(is_binary)), to_string)(input)
}

fn black_hole_format(input: &[u8]) -> IResult<&[u8], String> {
    map_res(
        preceded(char('|'), take_while1(is_black_hole_format)),
        to_string,
    )(input)
}

fn from_base64(input: &[u8]) -> Result<String> {
    Ok(String::from_utf8(BASE64_STANDARD.decode(input)?)?)
}

fn to_u8(input: &[u8]) -> Result<u8> {
    Ok(to_string(input)?.parse::<u8>()?)
}

fn to_string(input: &[u8]) -> Result<String> {
    Ok(std::str::from_utf8(input)?.to_string())
}

const fn is_binary(chr: u8) -> bool {
    matches!(chr, b'0'..=b'1')
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
