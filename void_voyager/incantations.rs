use crate::{alert::send_discord_message, nexus::Nexus};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;
use tracing::{info, instrument, warn};

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "GET", path = "/voyager")
)]
pub async fn unveil(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
) -> Response {
    nexus.atlas.unveil()
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "GET", path = "/voyager/version")
)]
pub async fn beacon(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
) -> Response {
    let latest_endless_void_version = nexus.manifest.read().latest_endless_void_version();
    (StatusCode::OK, latest_endless_void_version).into_response()
}

#[instrument(
    name = "request",
    skip(nexus, sigils),
    fields(method = "GET", path = "/voyager/{keys}")
)]
pub async fn probe(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    Path(sigils): Path<String>,
) -> Response {
    nexus.atlas.probe(sigils)
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "POST", path = "/voyager")
)]
pub async fn inscribe(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sector: String,
) -> Response {
    if nexus.manifest.read().origin_is_banned(origin.ip()) {
        info!("banned");
        return StatusCode::FORBIDDEN.into_response();
    }
    nexus.atlas.inscribe(
        sector,
        origin.ip(),
        nexus.manifest.read().latest_format_version(),
        nexus.manifest.read().allowed_songs(),
    )
}

#[instrument(
    name = "request",
    skip(nexus, sigil),
    fields(method = "POST", path = "/voyager/orphanage")
)]
pub async fn adopt(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sigil: String,
) -> Response {
    match nexus.atlas.adopt(&sigil) {
        Err(response) => *response,
        Ok(response) => {
            match nexus.atlas.sector(&sigil) {
                None => warn!("could not send discord webhook because level not found!"),
                Some(sector) => {
                    send_discord_message(nexus.manifest.read().discord_webhook_urls(), &sector);
                }
            }
            response
        }
    }
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "PUT", path = "/voyager")
)]
pub async fn amend(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sector_and_sigil: String,
) -> Response {
    if nexus.manifest.read().origin_is_banned(origin.ip()) {
        info!("banned");
        return StatusCode::FORBIDDEN.into_response();
    }
    nexus.atlas.amend(
        sector_and_sigil,
        origin.ip(),
        nexus.manifest.read().latest_format_version(),
        nexus.manifest.read().allowed_songs(),
    )
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "DELETE", path = "/voyager")
)]
pub async fn expunge(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sigil: String,
) -> Response {
    nexus.atlas.expunge(sigil)
}
