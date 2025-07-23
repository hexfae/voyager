//! The new, experimental parser for
//! [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
//! tiles and objects.
//!
//! [`parse_tiles`] and [`parse_objects`] are the main methods.

use crate::prelude::*;

use crate::sector::{
    Multiplier, Object, ObjectId, ObjectType, ParsedObjects, ParsedTiles, Tile, TileId, TileType,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use itertools::Itertools;
use nom::Parser;
use nom::sequence::{pair, preceded, terminated};
use nom::{
    IResult,
    branch::alt,
    bytes::complete::take_while_m_n,
    bytes::complete::{take, take_until, take_until1, take_while, take_while1},
    character::complete::{char, one_of},
    combinator::map_res,
    combinator::{all_consuming, opt},
    multi::{count, many1},
};
use std::str::FromStr;
use tracing::{info, warn};

// for documentation
#[allow(unused_imports)]
use crate::sector::BranefuckProgram;

/// Attempts to parse the input as a valid sequence of tiles.
///
/// # Errors
///
/// Returns an error if any tile was invalid. This includes an invalid
/// [`TileId`], an invalid [`TileType`], an invalid [`Multiplier`],
/// or some other invalid input.
pub fn parse_tiles(input: &str) -> Result<ParsedTiles> {
    let (_, tiles) = all_consuming(many1((tile_id, tile_type, multiplier)))
        .parse(input)
        .map_err(|why| {
            warn!("{why}");
            Error::InvalidTile(why.to_string())
        })?;
    let tiles = ParsedTiles::new(
        tiles
            .into_iter()
            .map(|(id, tile_type, multiplier)| Tile {
                id,
                tile_type,
                multiplier,
            })
            .collect_vec(),
    );
    if tiles.to_string() != input {
        warn!("mismatch!");
        info!("input: {input}");
        info!("recreated: {tiles}");
    }
    Ok(tiles)
}

/// Attempts to parse the input as a valid sequence of objects.
///
/// # Errors
///
/// Returns an error if any object was invalid. This includes an invalid
/// [`ObjectId`], an invalid [`ObjectType`], an invalid [`Multiplier`],
/// or some other invalid input.
pub fn parse_objects(input: &str) -> Result<ParsedObjects> {
    let (_, objects) = all_consuming(many1((object_id, object_type, multiplier)))
        .parse(input)
        .map_err(|why| {
            warn!("{why}");
            Error::InvalidObject(why.to_string())
        })?;
    let objects = ParsedObjects::new(
        objects
            .into_iter()
            .map(|(id, object_type, multiplier)| Object {
                id,
                object_type,
                multiplier,
            })
            .collect_vec(),
    );
    if objects.to_string() != input {
        warn!("mismatch!");
        info!("input: {input}");
        info!("recreated: {objects}");
    }
    Ok(objects)
}

/// Attempts to parse the input as a [`TileId`].
///
/// See [`TileId`] for all valid tile IDs.
fn tile_id(input: &str) -> IResult<&str, TileId> {
    map_res(take_while_m_n(2, 2, is_lowercase), TileId::from_str).parse(input)
}

/// Attempts to parse the input as an [`ObjectId`].
///
/// It attempts to take exactly 2 lowercase characters and parse them
/// into an [`ObjectId`].
///
/// Example of valid input: `em`
///
/// See [`ObjectId`] for details.
fn object_id(input: &str) -> IResult<&str, ObjectId> {
    map_res(take_while_m_n(2, 2, is_lowercase), ObjectId::from_str).parse(input)
}

/// Attempts to parse the input as a [`TileType`].
///
/// Examples of valid input: `03`, `17`.
///
/// A [`TileType`] is simply a wrapped [`u8`]. As such, any [`u8`] is a valid
/// [`TileType`]. However, the largeset [`TileType`] found in the wild is
/// is `17` (found as `wa17`).
fn tile_type(input: &str) -> IResult<&str, Option<TileType>> {
    let (remaining, tile_type) = take_while(is_digit)(input)?;
    let tile_type = tile_type.parse::<u8>().ok();
    tile_type.map_or(Ok((remaining, None)), |tile_type| {
        Ok((remaining, Some(TileType(tile_type))))
    })
}

/// Attempts to parse the input as an [`ObjectType`].
///
/// Examples of valid inputs:
/// 1. `2` ([`ObjectType::Direction`])
/// 2. `2!LGc6bGVlY2hfY291bnQsLS0=!MQ==!` ([`ObjectType::AddStatue`])
/// 3. `4aGVsbG8=!dGhlc2UgYXJl!bWVzc2FnZXM=!bG9s!` ([`ObjectType::Egg`])
/// 4.
///
/// See [`add_statue()`], [`egg()`], [`direction`], [`offset`], [`secret_exit`], and [`mural`] for details.
fn object_type(input: &str) -> IResult<&str, Option<ObjectType>> {
    opt(alt((
        // NOTE: offset must be before egg because otherwise egg will
        // parse only part of an offset and then the parser will error,
        // same with egg and direction
        add_statue,
        secret_exit,
        offset,
        egg,
        direction,
        mural,
    )))
    .parse(input)
}

/// Attempts to parse the input as a [`Multiplier`].
///
/// Examples of valid input: `X3`, `X17`.
///
/// A [`Multiplier`] is simply a wrapped [`u8`]. As such, any [`u8`] is a valid
/// [`Multiplier`]. However, multipliers are prefixed by `X`, so the input
/// must begin with `X` in order to be valid.
fn multiplier(input: &str) -> IResult<&str, Option<Multiplier>> {
    let (remaining, multiplier) = opt(preceded(char('X'), take_while(is_digit))).parse(input)?;
    multiplier.map_or(Ok((remaining, None)), |multiplier| {
        let multiplier = multiplier.parse::<u8>().ok();
        multiplier.map_or(Ok((remaining, None)), |multiplier| {
            Ok((remaining, Some(Multiplier(multiplier))))
        })
    })
}

/// Attempts to parse the input as an [`ObjectType::Egg`].
///
/// The first character must be a number (may be 0). This number decides how many
/// Base64-encoded, `!`-terminated messages follow it.
///
/// Examples of valid input:
/// 1. `0`
/// 2. `1aGVsbG8=!`
/// 3. `4aGk=!aGV5!eW8=!aGFoYQ==!`
fn egg(input: &str) -> IResult<&str, ObjectType> {
    let (remaining, number_of_messages) =
        map_res(take(1u8), |n: &str| n.parse::<usize>()).parse(input)?;
    let (remaining, messages) = count(
        take_until_termination_character_then_decode_base64,
        number_of_messages,
    )
    .parse(remaining)?;
    let messages = ObjectType::egg(messages);
    Ok((remaining, messages))
}

/// Attempts to parse the input as an [`ObjectType::Direction`].
///
/// It attempts to take as many digits as possible (at least 1) and
/// parse them into an [`ObjectType::Direction`].
///
/// Example of valid input: `2`
///
/// See [`ObjectType::direction()`] for details.
fn direction(input: &str) -> IResult<&str, ObjectType> {
    map_res(take_while1(is_digit), ObjectType::direction).parse(input)
}

/// Attempts to parse the input as an [`ObjectType::Offset`].
///
/// It attempts to take two signed integers terminated by `!`.
///
/// Example of valid input: `10!-5!`
///
/// See [`ObjectType::offset()`] for details.
fn offset(input: &str) -> IResult<&str, ObjectType> {
    map_res(
        pair(
            terminated((opt(char('-')), take_while1(is_digit)), char('!')),
            terminated((opt(char('-')), take_while1(is_digit)), char('!')),
        ),
        ObjectType::offset,
    )
    .parse(input)
}

/// Attempts to parse the input as an [`ObjectType::SecretExit`].
///
/// It attempts to take a single digit for its effect, and two signed integers for the offset,
/// all terminated by a `!` character.
///
/// Example of valid input: `2!10!-5!`
///
/// See [`ObjectType::secret_exit()`] for details.
fn secret_exit(input: &str) -> IResult<&str, ObjectType> {
    map_res(
        (
            terminated(take(1usize), char('!')),
            terminated((opt(char('-')), take_while1(is_digit)), char('!')),
            terminated((opt(char('-')), take_while1(is_digit)), char('!')),
        ),
        ObjectType::secret_exit,
    )
    .parse(input)
}

/// Attempts to parse the input as an [`ObjectType::Mural`].
///
/// It attempts to read an unsigned integer as for the brand, and a Base64-encoded string for the message,
/// both terminated by a `!` character.
///
/// Example of valid input: `1007159424!dm95YWdlcg==!`
///
/// See [`ObjectType::mural()`] for details.

fn mural(input: &str) -> IResult<&str, ObjectType> {
    map_res(
        pair(
            take_until_termination_character,
            take_until_termination_character_then_decode_base64,
        ),
        ObjectType::mural,
    )
    .parse(input)
}

/// Attempts to parse the input as an [`ObjectType::AddStatue`].
///
/// An [`ObjectType::AddStatue`] is prefixed by a `1` or `2`, followed by 2
/// Base64-encoded, `!`-terminated strings.
///
/// Examples of valid input:
/// 1!bGVlY2hfY291bnQ=!Mw==!
/// 2!LGc6bGVlY2hfY291bnQsLS0=!MQ==!
///
fn add_statue(input: &str) -> IResult<&str, ObjectType> {
    map_res(
        pair(
            terminated(one_of("12"), char('!')),
            pair(
                take_until_termination_character_then_decode_base64,
                take_until_termination_character_then_decode_base64,
            ),
        ),
        ObjectType::add_statue,
    )
    .parse(input)
}

/// Attempts to take characters until `!` is found.
///
/// Example of valid input: `[->-<]>?.!`
///
/// This is used for the
/// [Branefuck program](https://github.com/Skirlez/void-stranger-endless-void/wiki/Branefuck)
/// parameter of type 2 Add statues.
///
/// Unused as of format version 3 (Since then, Branefuck programs are Base64-encoded)
fn take_until_termination_character(input: &str) -> IResult<&str, &str> {
    terminated(take_until1("!"), char('!')).parse(input)
}

/// Attempts to take characters until `!` is found, then Base64-decodes them.
///
/// Example of valid input: `cGxheWVyX3g=!`
///
/// This is used in many places. For example, most parameters of Add statues
/// are `!`-terminated, as well as all messages of eggs.
fn take_until_termination_character_then_decode_base64(input: &str) -> IResult<&str, String> {
    terminated(map_res(take_until("!"), decode_base64), char('!')).parse(input)
}

/// Attempts to Base64-decode the input.
///
/// Returns [`Error::InvalidObjects`] on invalid input, since only objects use
/// Base64.
///
/// Example of valid input: `aGVsbG8=` (or any valid Base64, really)
fn decode_base64(input: &str) -> Result<String> {
    String::from_utf8(
        BASE64_STANDARD
            .decode(input)
            .map_err(|why| Error::InvalidObject(why.to_string()))?,
    )
    .map_err(|why| Error::InvalidObject(why.to_string()))
}

/// Tests if the character is an ASCII digit: `0-9`.
///
/// This function exists because `nom`'s built-in function takes in a [`u8`]
/// instead of a [`prim@char`].
const fn is_digit(input: char) -> bool {
    input.is_ascii_digit()
}

/// Tests if the character is a lowercase ASCII character: `a-z`.
///
/// This function exists because `nom`'s built-in function takes in a [`u8`]
/// instead of a [`prim@char`].
const fn is_lowercase(input: char) -> bool {
    input.is_ascii_lowercase()
}
