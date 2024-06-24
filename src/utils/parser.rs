//! The new, experimental parser for Endless Void tiles and objects.

use crate::prelude::*;

use crate::utils::level::{
    Multiplier, Object, ObjectId, ObjectType, ParsedObjects, ParsedTiles, Tile, TileId, TileType,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::take_while_m_n,
    bytes::complete::{take, take_until1, take_while, take_while1},
    character::complete::char,
    combinator::map_res,
    combinator::{all_consuming, opt},
    multi::{count, many1},
    sequence::{pair, preceded, terminated, tuple},
    IResult,
};
use tracing::{info, warn};

// for documentation
#[allow(unused_imports)]
use crate::utils::level::{BranefuckProgram, DestroyValue, InputValue};

impl ParsedTiles {
    /// Attempts to parse the input as a valid sequence of tiles.
    ///
    /// # Errors
    /// Returns an error if any tile was invalid. This includes an invalid
    /// [`TileId`], an invalid [`TileType`], an invalid [`Multiplier`],
    /// or some other invalid input.
    pub fn parse(input: &str) -> Result<Self> {
        let (_, tiles) = all_consuming(many1(tuple((tile_id, tile_type, multiplier))))(input)
            .map_err(|why| {
                warn!("{why}");
                Error::InvalidTiles
            })?;
        let tiles = Self(
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
        };
        Ok(tiles)
    }
}

impl ParsedObjects {
    /// Attempts to parse the input as a valid sequence of objects.
    ///
    /// # Errors
    /// Returns an error if any object was invalid. This includes an invalid
    /// [`ObjectId`], an invalid [`ObjectType`], an invalid [`Multiplier`],
    /// or some other invalid input.
    pub fn parse(input: &str) -> Result<Self> {
        let (_, objects) = all_consuming(many1(tuple((object_id, object_type, multiplier))))(input)
            .map_err(|why| {
                warn!("{why}");
                Error::InvalidObjects
            })?;
        let objects = Self(
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
        };
        Ok(objects)
    }
}

/// Attempts to parse the input as a [`TileId`].
///
/// See [`TileId`] for all valid tile IDs.
fn tile_id(input: &str) -> IResult<&str, TileId> {
    map_res(take_while_m_n(2, 2, is_lowercase), TileId::try_from)(input)
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
    map_res(take_while_m_n(2, 2, is_lowercase), ObjectId::try_from)(input)
}

/// Attempts to parse the input as an [`ObjectType`].
///
/// Examples of valid inputs:
/// 1. `2` ([`ObjectType::Direction`])
/// 2. `1bGVlY2hfY291bnQ=!Mw==!` ([`ObjectType::AddStatue1`])
/// 3. `2bGVlY2hfY291bnQ=!Mg==!MQ==![->-<]>?.!` ([`ObjectType::AddStatue2`])
/// 4. `4aGVsbG8=!dGhlc2UgYXJl!bWVzc2FnZXM=!bG9s!` ([`ObjectType::Egg`])
///
/// See [`add_statue()`], [`egg()`], and [`direction`] for details.
fn object_type(input: &str) -> IResult<&str, Option<ObjectType>> {
    opt(alt((add_statue, egg, direction)))(input)
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
    map_res(take_while1(is_digit), ObjectType::direction)(input)
}

/// Attempts to take characters until `!` is found.
///
/// Example of valid input: `[->-<]>?.!`
///
/// This is used for the Branefuck program parameter of type 2 Add statues.
fn take_until_termination_character(input: &str) -> IResult<&str, &str> {
    terminated(take_until1("!"), char('!'))(input)
}

/// Attempts to take characters until `!` is found, then Base64-decodes them.
///
/// Example of valid input: `cGxheWVyX3g=!`
///
/// This is used in many places. For example, most parameters of Add statues
/// are `!`-terminated, as well as all messages of eggs.
fn take_until_termination_character_then_decode_base64(input: &str) -> IResult<&str, String> {
    terminated(map_res(take_until1("!"), decode_base64), char('!'))(input)
}

/// Appends a [`&str`] to a [`Vec`] of [`String`]s.
///
/// This is a workaround. In [`add_statue2()`], a `(Vec<String>, &str)` is returned,
/// but `ObjectType::add_statue()` takes in a `Vec<String>`. This function simply turns
/// the `(Vec<String>, &str)` into `Vec<String>` by appending the `&str`. Additionally,
/// `nom`'s `map_res()` function takes in a function that returns a `Result<T, E>`, so
/// this function wraps the result in a `Result<T, E>`.
// map_res needs a function that returns a result
#[allow(clippy::unnecessary_wraps)]
fn append_branefuck_to_parameters(input: (Vec<String>, &str)) -> Result<Vec<String>> {
    Ok([input.0, vec![input.1.into()]].concat())
}

/// Attempts to parse the input as an [`ObjectType::AddStatue1`].
///
/// An [`ObjectType::AddStatue1`] is prefixed by a `1`, followed by an [`InputValue`]
/// and a [`DestroyValue`], both Base64-encoded and `!`-terminated.
///
/// Example of valid input: `1cGxheWVyX3g=!Ng==!`
fn add_statue1(input: &str) -> IResult<&str, ObjectType> {
    preceded(
        char('1'),
        map_res(
            count(take_until_termination_character_then_decode_base64, 2),
            ObjectType::add_statue,
        ),
    )(input)
}

/// Attempts to parse the input as an [`ObjectType::AddStatue2`].
///
/// An [`ObjectType::AddStatue2`] is prefixed by a `2`, followed by two [`InputValue`]s,
/// a [`DestroyValue`], and a [`BranefuckProgram`]. The first 3 parameters are
/// Base64-encoded, and all are `!`-terminated.
///
/// Example of valid input: `2bGVlY2hfY291bnQ=!Mg==!MQ==![->-<]>?.!`
fn add_statue2(input: &str) -> IResult<&str, ObjectType> {
    preceded(
        char('2'),
        map_res(
            map_res(
                pair(
                    count(take_until_termination_character_then_decode_base64, 3),
                    take_until_termination_character,
                ),
                append_branefuck_to_parameters,
            ),
            ObjectType::add_statue,
        ),
    )(input)
}

/// Attempts to parse the input as either type of Add statue.
///
/// See [`add_statue1()`] and [`add_statue2()`] for details.
fn add_statue(input: &str) -> IResult<&str, ObjectType> {
    alt((add_statue1, add_statue2))(input)
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
    let (remaining, number_of_messages) = map_res(take(1u8), |n: &str| n.parse::<usize>())(input)?;
    let (remaining, messages) = count(
        take_until_termination_character_then_decode_base64,
        number_of_messages,
    )(remaining)?;
    let messages = ObjectType::egg(messages);
    Ok((remaining, messages))
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
            .map_err(|_| Error::InvalidObjects)?,
    )
    .map_err(|_| Error::InvalidObjects)
}

/// Attempts to parse the input as a [`TileType`].
///
/// Examples of valid input: `03`, `17`.
///
/// A [`TileType`] is simply a wrapped [`u8`]. As such, any u8 is a valid
/// [`TileType`]. However, the largeset [`TileType`] found in the wild is
/// is `17` (found as `wa17`).
fn tile_type(input: &str) -> IResult<&str, Option<TileType>> {
    let (remaining, tile_type) = take_while(is_digit)(input)?;
    let tile_type = tile_type.parse::<u8>().ok();
    tile_type.map_or(Ok((remaining, None)), |tile_type| {
        Ok((remaining, Some(TileType(tile_type))))
    })
}

/// Attempts to parse the input as a [`Multiplier`].
///
/// Examples of valid input: `X3`, `X17`.
///
/// A [`Multiplier`] is simply a wrapped [`u8`]. As such, any u8 is a valid
/// [`Multiplier`]. However, multipliers are prefixed by `X`, so the input
/// must begin with `X` in order to be valid.
fn multiplier(input: &str) -> IResult<&str, Option<Multiplier>> {
    let (remaining, multiplier) = opt(preceded(char('X'), take_while(is_digit)))(input)?;
    multiplier.map_or(Ok((remaining, None)), |multiplier| {
        let multiplier = multiplier.parse::<u8>().ok();
        multiplier.map_or(Ok((remaining, None)), |multiplier| {
            Ok((remaining, Some(Multiplier(multiplier))))
        })
    })
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
