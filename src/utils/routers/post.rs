//! Routers for the POST HTTP method.

use crate::prelude::*;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
};
use std::net::SocketAddr;
use tracing::info;
use ulid::Ulid;

/// Stages a level for uploading (if valid) and returns
/// its key. An anti-orphan check [`orphanage`] is necessary.
///
/// See [`Data`] for details on level format.
///
/// Returns 201 CREATED and a ULID key if successful. Returns 400 BAD REQUEST if
/// the level was invalid.
pub async fn post(
    State(db): State<SharedAppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    level: String,
) -> Result<(StatusCode, String)> {
    let addr = addr.ip();
    info!("POST sent by {addr}: {level}");

    let level = Level::new(level);
    let mut parsed = level.into_parsed()?;
    parsed.set_dates_to_now();
    info!("POST completed:\n{parsed}");

    let level = parsed.into_level();
    let id = Ulid::new();

    db.insert_orphan(id, level);
    Ok((StatusCode::CREATED, id.to_string()))
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
/// Yes, this whole thing is probably unnecessary, but
/// it was requested by the Endless Void developer.
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

    let ssn = key.parse()?;
    db.adopt_orphan(&ssn)?;

    info!("ADOPTION successful!");
    Ok(StatusCode::OK)
}
