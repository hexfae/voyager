//! The backend for the Web UI.

use crate::prelude::*;

use axum::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use inquire::{min_length, Password, Text};
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use std::fs::{read, write};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A Web UI user. Used for administrative tasks.
pub struct User {
    /// User's id. Always 1.
    ///
    /// This is because the current implementation
    /// only allows for one user (an admin).
    id: i64,
    /// User's username.
    pub username: String,
    /// User's password hash.
    password_hash: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
/// Web UI backend.
pub struct Backend {
    /// All Web UI users.
    ///
    /// Currently, there can only be one (an admin).
    users: std::collections::HashMap<i64, User>,
}

#[derive(Clone, Deserialize)]
/// A user's credentials, used for authentication.
pub struct Credentials {
    /// User's username.
    pub username: String,
    /// User's password.
    ///
    /// Note: This is never stored nor logged. This
    /// is immediately hashed and then dropped.
    pub password: String,
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

impl Backend {
    /// Attempts to load a Web UI user from
    /// `voyager/webui.db`. If it fails (likely due
    /// to it not yet existing), it instead creates
    /// a new one using `Self::new()`, which will
    /// ask for a username and password.
    ///
    /// # Errors
    ///
    /// Returns an error if a Web UI user is found, but
    /// deserializing it fails. Most likely, some
    /// data structure had a breaking change (or
    /// the file is corrupted).
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    pub fn try_load() -> Result<Self> {
        debug!("Web UI user is opening...");
        let input = read("voyager/webui.db");
        input.map_or_else(
            |_| {
                info!("Existing Web UI user not found! Please create one...");
                Self::new()
            },
            |bytes| {
                debug!("Web UI user opened. Web UI user loading...");
                let backend = Self::from(&bytes);
                match backend {
                    Err(why) => {
                        error!("Web UI user could not be loaded! {why}");
                        Err(why)
                    }
                    Ok(backend) => {
                        // why does Values have a last() method but not a first() method?
                        let user = backend.users.values().last().cloned();
                        user.map_or_else(
                            || {
                                error!("Web UI user could not be found!");
                                Err(Error::WebUI)
                            },
                            |user| {
                                info!("Web UI user loaded: {}.", user.username);
                                Ok(backend)
                            },
                        )
                    }
                }
            },
        )
    }

    /// Attempts to save itself to `voyager/webui.db`.
    ///
    /// If it fails (likely due to file permissions),
    /// it will log a warning and keep running.
    // it is really not that complex
    #[allow(clippy::cognitive_complexity)]
    fn save(&self) {
        debug!("Web UI user is serializing...");
        match bincode::serialize(&self) {
            Ok(bytes) => {
                debug!("Web UI user serialized. Web UI is saving...");
                if let Err(why) = write("voyager/webui.db", bytes) {
                    warn!("Web UI user could not be saved: {why}");
                } else {
                    debug!("Web UI user saved.");
                };
            }
            Err(why) => warn!("Web UI could not be serialized: {why}"),
        }
    }

    /// Attempts to deserialize a Web UI user from bytes.
    ///
    /// # Errors
    ///
    /// This function will return an error if deserializing
    /// it fails. Most likely, some data structure had a
    /// breaking change (or the file is corrupted).
    fn from(webui: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(webui)?)
    }

    /// Asks for a username and password on the CLI.
    ///
    /// The name must be at least 2 characters long.
    /// The password must be at least 8 characters long.
    fn new() -> Result<Self> {
        let username = Text::new("username:")
            .with_validator(min_length!(2))
            .prompt()?;
        let password = Password::new("password:")
            .with_validator(min_length!(8))
            .prompt()?;
        let login = Self {
            users: std::collections::HashMap::from([(
                1,
                User {
                    id: 1,
                    username,
                    password_hash: generate_hash(password),
                },
            )]),
        };
        login.save();
        Ok(login)
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        Credentials {
            username, password, ..
        }: Self::Credentials,
    ) -> Result<Option<Self::User>> {
        let user = self
            .users
            .values()
            .find(|user| user.username == username)
            .cloned();

        tokio::task::spawn_blocking(|| {
            Ok(user.filter(|user| verify_password(password, &user.password_hash).is_ok()))
        })
        .await?
    }

    async fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        Ok(self.users.get(user_id).cloned())
    }
}
