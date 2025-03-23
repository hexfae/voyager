use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dashmap::{DashMap, DashSet};
use miette::Diagnostic;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::{
    fs::{read, read_to_string, write},
    net::IpAddr,
    sync::Arc,
};
use time::OffsetDateTime;
use tracing::{debug, info, warn};
use void_codex::{Sector, Sigil};

const CONFIG_PATH: &str = "voyager/config.ron";

#[derive(Debug, Clone)]
pub struct Nexus {
    pub atlas: Arc<Atlas>,
    pub manifest: Arc<RwLock<Manifest>>,
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
    #[serde(rename = "banned_ips")]
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

#[derive(Debug, Snafu, Diagnostic)]
pub enum NexusError {
    #[snafu(transparent)]
    #[diagnostic(transparent)]
    Atlas { source: AtlasError },
    #[snafu(transparent)]
    #[diagnostic(transparent)]
    Manifest { source: ManifestError },
}

#[derive(Debug, Snafu, Diagnostic)]
#[allow(clippy::enum_variant_names)] // due to clashing names with ManifestError
pub enum AtlasError {
    #[snafu(display("Failed to open the levels file"))]
    #[diagnostic(
        code(void_voyager::nexus::AtlasError::ReadAtlas),
        help("Do you have read permissions for the file?")
    )]
    ReadAtlas { source: std::io::Error },
    #[snafu(display("Failed to deserialize the opened levels file"))]
    #[diagnostic(
        code(void_voyager::nexus::AtlasError::DecodeAtlas),
        help("Was there a breaking change? Let me know!")
    )]
    DecodeAtlas { source: Box<bincode::ErrorKind> },
    #[snafu(display("Failed to serialize the levels"))]
    #[diagnostic(
        code(void_voyager::nexus::AtlasError::EncodeAtlas),
        help("You're on your own for this one.")
    )]
    EncodeAtlas { source: Box<bincode::ErrorKind> },
    #[snafu(display("Failed to write the levels file"))]
    #[diagnostic(
        code(void_voyager::nexus::AtlasError::WriteAtlas),
        help("Do you have write permissions for the file?")
    )]
    WriteAtlas { source: std::io::Error },
}

#[derive(Debug, Snafu, Diagnostic)]
#[allow(clippy::enum_variant_names)] // due to clashing names with AtlasError
pub enum ManifestError {
    #[snafu(display("Failed to open the config file"))]
    #[diagnostic(
        code(void_voyager::nexus::ManifestError::ReadManifest),
        help("Do you have read permissions for the file?")
    )]
    ReadManifest { source: std::io::Error },
    #[snafu(display("Failed to deserialize the opened config"))]
    #[diagnostic(
        code(void_voyager::nexus::ManifestError::DecodeManifest),
        help("Did you forget a parenthesis?")
    )]
    DecodeManifest { source: ron::de::SpannedError },
    #[snafu(display("Failed to serialize the config"))]
    #[diagnostic(
        code(void_voyager::nexus::ManifestError::EncodeManifest),
        help("You're on your own for this one.")
    )]
    EncodeManifest { source: ron::error::Error },
    #[snafu(display("Failed to write the config file"))]
    #[diagnostic(
        code(void_voyager::nexus::ManifestError::WriteManifest),
        help("Do you have write permissions for the file?")
    )]
    WriteManifest { source: std::io::Error },
}

fn try_open_file_bytes(path: impl AsRef<str>) -> Result<Option<Vec<u8>>, std::io::Error> {
    match read(path.as_ref()) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(why) => {
            if why.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(why)
            }
        }
    }
}

fn try_open_file_string(path: impl AsRef<str>) -> Result<Option<String>, std::io::Error> {
    match read_to_string(path.as_ref()) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(why) => {
            if why.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(why)
            }
        }
    }
}

impl Nexus {
    pub fn try_load() -> Result<Self, NexusError> {
        Ok(Self {
            atlas: Arc::new(Atlas::try_load()?),
            manifest: Arc::new(RwLock::new(Manifest::try_load()?)),
        })
    }

    pub fn try_save(&self) -> Result<(), NexusError> {
        self.atlas.try_save()?;
        self.manifest.read().try_save()?;
        Ok(())
    }

    pub fn vaporize(&self, origin: IpAddr) {
        self.atlas.vaporize(origin);
        self.manifest.read().vaporize(origin);
    }
}

impl Atlas {
    pub fn try_load() -> Result<Self, AtlasError> {
        let bytes = try_open_file_bytes("voyager/levels.db").context(ReadAtlasSnafu)?;
        match bytes {
            Some(bytes) => Ok(bincode::deserialize(&bytes).context(DecodeAtlasSnafu)?),
            None => Ok(Self::default()),
        }
    }

    pub fn try_save(&self) -> Result<(), AtlasError> {
        let bytes = bincode::serialize(&self).context(EncodeAtlasSnafu)?;
        write("voyager/levels.db", bytes).context(WriteAtlasSnafu)?;
        Ok(())
    }

    pub fn backup(&self) {
        let now = OffsetDateTime::now_utc()
            // 2025-03-19
            .date()
            .to_string();
        let path = format!("voyager/backups/{now}.db");
        debug!("Backup is being created at {path}...");

        match bincode::serialize(&self) {
            Err(why) => {
                warn!("Backup could not be created: {why}");
            }
            Ok(bytes) => {
                debug!("Backup created. Backup is saving...");
                let len = bytes.len();
                if let Err(why) = write(path, bytes) {
                    warn!("Backup could not be saved: {why}");
                } else {
                    debug!("Backup saved: {len} bytes.");
                }
            }
        }
    }

    pub fn name_and_author_collision_found(&self, new: &Sector) -> bool {
        self.sectors.iter().any(|sector| {
            sector.name() == new.name()
                && sector.author() == new.author()
                && sector.sigil() != new.sigil()
        })
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

    pub fn adopt(&self, sigil: impl Into<String>) -> Result<Response, Box<Response>> {
        let Ok(sigil) = sigil.into().parse::<Sigil>() else {
            return Err(Box::new(StatusCode::BAD_REQUEST.into_response()));
        };
        let Some((_, sector)) = self.orphans.remove(&sigil) else {
            return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
        };
        info!("{} by {} adopted", sector.name(), sector.author());
        self.sectors.insert(sigil, sector);
        Ok(StatusCode::NO_CONTENT.into_response())
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
        }
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

    pub fn vaporize(&self, origin: IpAddr) {
        self.sectors
            .iter()
            .filter(|element| element.origin() == origin)
            .for_each(|sector| {
                self.expunge(sector.sigil());
            });
    }

    pub fn sector(&self, sigil: impl AsRef<str>) -> Option<Sector> {
        let Ok(sigil) = sigil.as_ref().parse::<Sigil>() else {
            return None;
        };
        self.sectors
            .get(&sigil)
            .map(|sector| sector.value().clone())
    }

    pub fn sectors(&self) -> Vec<Sector> {
        self.sectors
            .iter()
            .map(|sector| sector.value().clone())
            .collect()
    }
}

impl Manifest {
    pub fn try_load() -> Result<Self, ManifestError> {
        let string = try_open_file_string(CONFIG_PATH).context(ReadManifestSnafu)?;
        match string {
            None => Ok(Self::default()),
            Some(string) => Ok(ron::de::from_str(&string).context(DecodeManifestSnafu)?),
        }
    }

    pub fn try_save(&self) -> Result<(), ManifestError> {
        let string = ron::ser::to_string_pretty(&self, ron::ser::PrettyConfig::default())
            .context(EncodeManifestSnafu)?;
        write(CONFIG_PATH, string).context(WriteManifestSnafu)?;
        Ok(())
    }

    pub fn reload(&mut self) -> Result<(), ManifestError> {
        let new = Self::try_load()?;
        *self = new;
        Ok(())
    }

    pub fn vaporize(&self, origin: IpAddr) {
        self.banned_origins.0.insert(origin);
        self.try_save().ok();
    }

    pub fn origin_is_banned(&self, origin: IpAddr) -> bool {
        self.banned_origins.0.contains(&origin)
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn allowed_songs(&self) -> Vec<String> {
        self.allowed_songs.0.clone()
    }

    pub const fn latest_format_version(&self) -> u8 {
        self.latest_format_version.0
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn latest_endless_void_version(&self) -> String {
        self.latest_endless_void_version.0.clone()
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn discord_webhook_urls(&self) -> Vec<String> {
        self.discord_webhook_urls.0.clone()
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
