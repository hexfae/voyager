use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use std::{
    fs::{read, read_to_string},
    net::IpAddr,
    sync::Arc,
};
use tracing::{info, warn};
use void_codex::{Sector, Sigil};

use crate::error::Error;

type Result<T, E = Error> = std::result::Result<T, E>;

// maybe AstralIndex
#[derive(Debug, Clone)]
pub struct Nexus {
    pub atlas: Arc<Atlas>,
    pub manifest: Arc<Manifest>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Atlas {
    sectors: DashMap<Sigil, Sector>,
    orphans: DashMap<Sigil, Sector>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    allowed_songs: AllowedSongs,
    #[serde(default)]
    latest_format_version: LatestFormatVersion,
    #[serde(default)]
    latest_endless_void_version: LatestEndlessVoidVersion,
    #[serde(default)]
    discord_webhook_urls: DiscordWebhookUrls,
    #[serde(default)]
    banned_origins: BannedOrigins,
}

/// The list of allowed songs.
///
/// See [`DEFAULT_ALLOWED_SONGS`] for the list of defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedSongs(pub Vec<String>);

/// The latest level format version.
///
/// See [`DEFAULT_FORMAT_VERSION`] for the default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestFormatVersion(pub u8);

/// The latest
/// [Endless Void](https://github.com/Skirlez/void-stranger-endless-void)
/// version.
///
/// See [`DEFAULT_ENDLESS_VOID_VERSION`] for the default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestEndlessVoidVersion(pub String);

/// The list of Discord webhook URLs used for level upload messages.
///
/// Voyager can optionally send a Discord message using
/// the provided webhook URLs when a level is uploaded.
///
/// The default is an empty [`Vec`], which will skip
/// trying to send the message altogether. Webhook URLs must
/// be set through the config. Voyager will attempt to send
/// to every configured webhook URL.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscordWebhookUrls(Vec<String>);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BannedOrigins(DashSet<IpAddr>);

impl Nexus {
    pub fn try_load() -> Result<Self> {
        Ok(Self {
            atlas: Arc::new(Atlas::try_load()?),
            manifest: Arc::new(Manifest::try_load()?),
        })
    }
}

impl Atlas {
    pub fn try_load() -> Result<Self> {
        read("voyager/config.ron").map_or_else(
            |_| Ok(Self::default()),
            |atlas| match bincode::deserialize(&atlas) {
                Ok(atlas) => Ok(atlas),
                Err(why) => Err(Error::ReadAtlas { source: why }),
            },
        )
    }

    pub fn name_and_author_collision_found(&self, new: &Sector) -> bool {
        self.sectors
            .iter()
            .find(|sector| new.name() == sector.name() && new.author() == sector.author())
            .is_some_and(|found| new.sigil() != found.sigil())
    }

    pub fn unveil(&self) -> Response {
        let sectors = self
            .sectors
            .iter()
            .map(|sector| sector.value().cipher())
            .collect::<Vec<String>>();
        info!("{} levels sent", sectors.len());
        let sectors = sectors.join(",");
        (StatusCode::OK, sectors).into_response()
    }

    pub fn probe(&self, sigils: impl AsRef<str>) -> Response {
        let sigils = sigils.as_ref().split(',').collect::<Vec<&str>>();
        let sigil_count = sigils.len();
        let parsed_sigils = sigils
            .into_iter()
            .filter_map(|sigil| sigil.parse::<Sigil>().ok());
        if sigil_count != parsed_sigils.clone().count() {
            info!("one or more key was invalid");
            return StatusCode::BAD_REQUEST.into_response();
        }
        let valid_sigils = parsed_sigils
            .map(|sigil| u8::from(self.sectors.contains_key(&sigil)).to_string())
            .collect::<String>();
        let valid_sigils_count = valid_sigils.chars().filter(|char| *char == '1').count();
        info!("{sigil_count} levels checked ({valid_sigils_count} valid)");
        (StatusCode::OK, valid_sigils).into_response()
    }

    pub fn inscribe(
        &self,
        cipher: impl Into<String> + AsRef<str>,
        origin: IpAddr,
        latest_version: impl Into<u8>,
        allowed_songs: impl AsRef<[String]>,
    ) -> Response {
        let Ok(sector) = Sector::inscribe(cipher, origin, latest_version, allowed_songs) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if self.name_and_author_collision_found(&sector) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let sigil = Sigil::new();
        info!("{} by {} up for adoption", sector.name(), sector.author());
        self.orphans.insert(sigil, sector);
        (StatusCode::CREATED, sigil).into_response()
    }

    pub fn adopt(&self, sigil: impl Into<String>) -> Response {
        let Ok(sigil) = sigil.into().parse::<Sigil>() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Some((_, sector)) = self.orphans.remove(&sigil) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        info!("{} by {} adopted", sector.name(), sector.author());
        self.sectors.insert(sigil, sector);
        StatusCode::NO_CONTENT.into_response()
    }

    pub fn amend(
        &self,
        cipher_and_sigil: impl Into<String>,
        origin: IpAddr,
        latest_version: impl Into<u8>,
        allowed_songs: impl AsRef<[String]>,
    ) -> Response {
        let cipher_and_sigil = cipher_and_sigil.into();
        let Some((cipher, sigil)) = cipher_and_sigil.rsplit_once('|') else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Ok(sigil) = sigil.parse::<Sigil>() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Ok(mut sector) = Sector::amend(cipher, sigil, origin, latest_version, allowed_songs)
        else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if self.name_and_author_collision_found(&sector) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let Some(old_sector) = self.sectors.get(&sigil) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        if sector.set_uploaded_from(&old_sector).is_err() {
            warn!("failed to set uploaded from old level");
        };
        info!("{} by {} edited", sector.name(), sector.author());
        self.sectors.insert(sigil, sector);
        StatusCode::NO_CONTENT.into_response()
    }

    pub fn expunge(&self, sigil: impl Into<String>) -> Response {
        let Ok(sigil) = sigil.into().parse::<Sigil>() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let Some((_, sector)) = self.sectors.remove(&sigil) else {
            return StatusCode::NOT_FOUND.into_response();
        };
        info!("{} by {}", sector.name(), sector.author());
        StatusCode::NO_CONTENT.into_response()
    }
}

impl Manifest {
    pub fn try_load() -> Result<Self> {
        read_to_string("voyager/config.ron").map_or_else(
            |_| Ok(Self::default()),
            |manifest| match ron::from_str::<Self>(&manifest) {
                Ok(manifest) => Ok(manifest),
                Err(why) => Err(Error::ReadManifest { source: why }),
            },
        )
    }

    pub fn origin_is_banned(&self, origin: IpAddr) -> bool {
        self.banned_origins.0.contains(&origin)
    }

    pub fn allowed_songs(&self) -> &[String] {
        &self.allowed_songs.0
    }

    pub const fn latest_format_version(&self) -> u8 {
        self.latest_format_version.0
    }

    pub fn latest_endless_void_version_response(&self) -> Response {
        let latest_endless_void_version = self.latest_endless_void_version.0.clone();
        info!("{latest_endless_void_version}");
        (StatusCode::OK, latest_endless_void_version).into_response()
    }

    pub fn latest_endless_void_version(&self) -> &str {
        &self.latest_endless_void_version.0
    }

    pub fn discord_webhook_urls(&self) -> &[String] {
        &self.discord_webhook_urls.0
    }
}

impl AllowedSongs {
    const DEFAULT_ALLOWED_SONGS: [&str; 18] = [
        "", // ambience
        "msc_001",
        "msc_dungeon_wings",
        "msc_beecircle",
        "msc_dungeongroove",
        "msc_013",
        "msc_gorcircle_lo",
        "msc_levcircle",
        "msc_escapewithfriend",
        "msc_cifcircle",
        "msc_006",
        "msc_beesong",
        "msc_themeofcif",
        "msc_monstrail",
        "msc_endless",
        "msc_stg_extraboss",
        "msc_rytmi2",
        "msc_test2",
    ];
}

impl LatestFormatVersion {
    const DEFAULT_LATEST_FORMAT_VERSION: u8 = 2;
}

impl LatestEndlessVoidVersion {
    const DEFAULT_LATEST_ENDLESS_VOID_VERSION: &str = "0.89";
}

impl Default for AllowedSongs {
    fn default() -> Self {
        Self(
            Self::DEFAULT_ALLOWED_SONGS
                .map(ToString::to_string)
                .to_vec(),
        )
    }
}

impl Default for LatestFormatVersion {
    fn default() -> Self {
        Self(Self::DEFAULT_LATEST_FORMAT_VERSION)
    }
}

impl Default for LatestEndlessVoidVersion {
    fn default() -> Self {
        Self(Self::DEFAULT_LATEST_ENDLESS_VOID_VERSION.into())
    }
}
