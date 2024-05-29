//! Log in.

use crate::prelude::*;

use askama_axum::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};

#[derive(Template)]
#[template(path = "login.html")]
/// Askama template for rendering the login page.
struct LoginPage;

/// The login page.
pub async fn get() -> impl IntoResponse {
    LoginPage
}

/// The function responsible for authenticating a login.
///
/// Returns 401 UNAUTHORIZED if the login is incorrect,
/// 500 INTERNAL SERVER ERROR if something went wrong,
/// or redirects to `/voyager/webui` if login succeeded.
pub async fn post(
    mut auth_session: AuthSession,
    Form(creds): Form<Credentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/voyager/webui").into_response()
}
