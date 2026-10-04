use crate::nexus::Nexus;
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
    fields(method = "GET", path = "/voyager/packs")
)]
pub async fn unveil(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
) -> Response {
    nexus.cluster_atlas.unveil()
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "GET", path = "/voyager/packs/detail/{save_name}")
)]
pub async fn detail(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    Path(save_name): Path<String>,
) -> Response {
    nexus.cluster_atlas.detail(save_name)
}

#[instrument(
    name = "request",
    skip(nexus, sigils),
    fields(method = "GET", path = "/voyager/packs/{keys}")
)]
pub async fn probe(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    Path(sigils): Path<String>,
) -> Response {
    nexus.cluster_atlas.probe(sigils)
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "POST", path = "/voyager/packs")
)]
pub async fn inscribe(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    cluster: String,
) -> Response {
    warn!("WOAH");
    if nexus.manifest.read().origin_is_banned(origin.ip()) {
        info!("banned");
        return StatusCode::FORBIDDEN.into_response();
    }
    nexus.cluster_atlas.inscribe(
        cluster,
        origin.ip(),
        nexus.manifest.read().latest_pack_format_version(),
        nexus.manifest.read().latest_format_version(),
        nexus.manifest.read().allowed_songs(),
    )
}

#[instrument(
    name = "request",
    skip(nexus, sigil),
    fields(method = "POST", path = "/voyager/packs/orphanage")
)]
pub async fn adopt(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sigil: String,
) -> Response {
    match nexus.cluster_atlas.adopt(&sigil) {
        Err(response) => *response,
        Ok(response) => {
            // TODO: discord webhook notifications for packs
            // Should we do something equivalent to levels with the thumbnail?
            response
        }
    }
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "PUT", path = "/voyager/packs")
)]
pub async fn amend(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    cluster_and_sigil: String,
) -> Response {
    if nexus.manifest.read().origin_is_banned(origin.ip()) {
        info!("banned");
        return StatusCode::FORBIDDEN.into_response();
    }
    nexus.cluster_atlas.amend(
        cluster_and_sigil,
        origin.ip(),
        nexus.manifest.read().latest_pack_format_version(),
        nexus.manifest.read().latest_format_version(),
        nexus.manifest.read().allowed_songs(),
    )
}

#[instrument(
    name = "request",
    skip(nexus),
    fields(method = "DELETE", path = "/voyager/packs")
)]
pub async fn expunge(
    State(nexus): State<Nexus>,
    ConnectInfo(origin): ConnectInfo<SocketAddr>,
    sigil: String,
) -> Response {
    nexus.cluster_atlas.expunge(sigil)
}
