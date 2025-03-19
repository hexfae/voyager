use axum_login::{AuthUser, AuthnBackend, UserId};
use inquire::{Password, Text, min_length};
use miette::Diagnostic;
use password_auth::{generate_hash, verify_password};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
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

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(display("Failed to authenticate Web UI user"))]
#[diagnostic(
    code(void_voyager::webui::backend::Backend::try_load),
    help("Is the login information correct?")
)]
pub struct AuthenticationError;

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Debug, Snafu, Diagnostic)]
#[allow(clippy::enum_variant_names)]
pub enum BackendError {
    #[snafu(display("Failed to load Web UI user"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::try_load))]
    LoadUserError { source: AuthenticationError },
    #[snafu(display("Failed to create Web UI user"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::new))]
    NewUserError { source: UserCreationError },
    #[snafu(display("Failed to deserialize Web UI user from bytes"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::from))]
    DeserializeUserError { source: WebUiError },
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
    pub fn try_load() -> Result<Self, BackendError> {
        debug!("Web UI user is opening...");
        let input = read("voyager/webui.db");
        input.map_or_else(
            |_| {
                info!("Existing Web UI user not found! Please create one...");
                Self::new().context(NewUserSnafu)
            },
            |bytes| {
                debug!("Web UI user opened. Web UI user loading...");
                let backend = Self::from(&bytes);
                match backend {
                    Err(why) => {
                        error!("Web UI user could not be loaded! {why}");
                        Err(why).context(DeserializeUserSnafu)
                    }
                    Ok(backend) => {
                        let user = backend.users.values().next().cloned();
                        user.map_or_else(
                            || {
                                error!("Web UI user could not be found!");
                                Err(AuthenticationError).context(LoadUserSnafu)
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
                }
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
    fn from(webui: &[u8]) -> Result<Self, WebUiError> {
        bincode::deserialize(webui).context(DeserializeSnafu)
    }

    /// Asks for a username and password on the CLI.
    ///
    /// The name must be at least 2 characters long.
    /// The password must be at least 8 characters long.
    fn new() -> Result<Self, UserCreationError> {
        let username = Text::new("username:")
            .with_validator(min_length!(2))
            .prompt()
            .context(UsernameSnafu)?;
        let password = Password::new("password:")
            .with_validator(min_length!(8))
            .prompt()
            .context(PasswordSnafu)?;
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

#[derive(Debug, Snafu, Diagnostic)]
pub enum UserCreationError {
    #[snafu(display("Failed to ask for Web UI username"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::new))]
    Username { source: inquire::InquireError },
    #[snafu(display("Failed to ask for Web UI password"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::new))]
    Password { source: inquire::InquireError },
}

#[derive(Debug, Snafu, Diagnostic)]
pub enum WebUiError {
    #[snafu(display("Failed to deserialize Web UI user from bytes"))]
    #[diagnostic(code(void_voyager::webui::backend::Backend::from))]
    DeserializeError { source: bincode::Error },
}

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(display("Failed to join Tokio task?"))]
#[diagnostic(
    code(void_voyager::webui::backend::Backend::authenticate),
    help("You're on your own for this one.")
)]
pub struct TokioJoinTaskError {
    source: tokio::task::JoinError,
}

#[async_trait::async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = TokioJoinTaskError;

    async fn authenticate(
        &self,
        Self::Credentials {
            username, password, ..
        }: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = self
            .users
            .values()
            .find(|user| user.username == username)
            .cloned();

        tokio::task::spawn_blocking(|| {
            Ok(user.filter(|user| verify_password(password, &user.password_hash).is_ok()))
        })
        .await
        .context(TokioJoinTaskSnafu)?
    }

    async fn get_user(
        &self,
        user_id: &UserId<Self>,
    ) -> std::result::Result<Option<Self::User>, Self::Error> {
        Ok(self.users.get(user_id).cloned())
    }
}
