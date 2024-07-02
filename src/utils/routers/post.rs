//! Routers for the POST HTTP method.

use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::{debug, info};

// for documentation
#[allow(unused_imports)]
use crate::utils::level::Data;

/// Stages a level for uploading (if valid) and returns
/// its key. An anti-orphan check [`orphanage`] is necessary.
///
/// See [`Data`] for details on level format.
///
/// Returns 201 CREATED and a [ULID](https://github.com/ulid/spec)
/// key if successful. Returns 400 BAD REQUEST if the level was invalid.
pub async fn post(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<(StatusCode, String)> {
    let addr = addr.ip();
    info!("POST sent by {addr}");
    debug!("{level}");
    if db.ip_is_banned(&addr) {
        info!("{addr} is banned! :(");
        return Err(Error::Banned);
    }

    let level = Level::new(level, addr);
    debug!("Level is parsing...");
    let try_parse = level.into_parsed(&db.config.read());
    let mut parsed = match try_parse {
        Ok(parsed) => parsed,
        Err(why) => {
            info!("POST failed! {why}");
            return Err(why);
        }
    };
    debug!("Level parsed.\n{parsed}");

    db.check_for_name_and_author_collisions(&parsed.name.0, &parsed.author.0, None)?;

    parsed.set_dates_to_now();
    info!("POST success: {} by {}", parsed.name, parsed.author);
    debug!("{parsed}");

    let level = parsed.into_level();
    let key = level.key.to_string();

    db.insert_orphan(level);
    debug!("Orphan inserted.");
    Ok((StatusCode::CREATED, key))
}

/// Moves a level from the orphan list to the level list.
///
/// To make sure that the client received and saved the key,
/// Voyager will wait to insert levels into the database until
/// it receives the level's key back, finally inserting the
/// level into the level list if successful. This is to combat
/// the possible immediate creation of orphan levels (ones
/// where the key is lost).
///
/// Returns 200 OK if successful. Returns 400 BAD REQUEST on
/// invalid key. Returns 404 NOT FOUND on valid key, but
/// no matching level.
pub async fn orphanage(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    key: String,
) -> Result<StatusCode> {
    let addr = addr.ip();
    info!("ADOPTION sent by {addr}");

    debug!("Key is parsing...");
    let ssn = match key.parse() {
        Ok(ssn) => ssn,
        Err(why) => {
            info!("ADOPTION failed: {why}");
            return Err(why);
        }
    };
    debug!("Key parsed.");

    if let Err(why) = db.adopt_orphan(&ssn) {
        info!("ADOPTION failed: {why}");
        return Err(why);
    };

    info!("ADOPTION success");
    Ok(StatusCode::OK)
}
