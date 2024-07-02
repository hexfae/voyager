//! Router for the PUT HTTP method.

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

/// Updates an already uploaded level in the database.
///
/// See [`Data`] for details on level format.
///
/// Returns 201 CREATED if successful. Returns 400 BAD REQUEST on invalid
/// level data. Returns 401 UNAUTHORIZED on invalid key. Returns 404 NOT
/// FOUND on if somehow, the level data and key are valid, but the key is
/// not associated with any uploaded level.
pub async fn put(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<StatusCode> {
    let addr = addr.ip();
    info!("PUT sent by {addr}");
    debug!("{level}");
    if db.ip_is_banned(&addr) {
        info!("{addr} is banned! :(");
        return Err(Error::Banned);
    }

    let level = match Level::new_from_put(&level, addr) {
        Ok(level) => level,
        Err(why) => {
            info!("PUT failed! {why}");
            return Err(why);
        }
    };
    let key = level.key;
    debug!("Level is parsing...");
    let try_parse = level.into_parsed(&db.config.read());
    let mut parsed = match try_parse {
        Ok(parsed) => parsed,
        Err(why) => {
            info!("PUT failed! {why}");
            return Err(why);
        }
    };
    debug!("Level parsed.\n{parsed}.");

    let old_level = match db.get(&key) {
        Ok(level) => level,
        Err(why) => {
            info!("PUT failed! {why}");
            return Err(why);
        }
    };

    db.check_for_name_and_author_collisions(
        &parsed.name.0,
        &parsed.author.0,
        Some(&old_level),
    )?;

    parsed.set_dates_to_now();
    debug!("Upload is being set from old level...");
    if let Err(why) = parsed.set_uploaded_from(old_level, &db.config.read()) {
        info!("PUT failed! {why}");
        return Err(why);
    };
    debug!("Upload set.");
    info!("PUT success: {} by {}", parsed.name, parsed.author);
    let level = parsed.into_level();
    db.insert(level);
    Ok(StatusCode::OK)
}
