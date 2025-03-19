//! Log in.

use crate::webui::backend::{Backend, Credentials};
use askama_axum::{Response, Template};
use axum::{Form, http::StatusCode, response::Redirect};
use axum_login::AuthSession;

#[derive(Template)]
#[template(path = "login.html")]
/// Askama template for rendering the login page.
struct LoginPage;

/// The login page.
pub async fn get() -> Response {
    askama_axum::into_response(&LoginPage)
}

/// The function responsible for authenticating a login.
///
/// Returns 401 UNAUTHORIZED if the login is incorrect,
/// 500 INTERNAL SERVER ERROR if something went wrong,
/// or redirects to `/voyager/webui` if login succeeded.
pub async fn post(
    mut auth_session: AuthSession<Backend>,
    Form(creds): Form<Credentials>,
) -> axum::response::Response {
    let user = match auth_session.authenticate(creds.clone()).await {
        Ok(Some(user)) => user,
        Ok(None) => return axum::response::IntoResponse::into_response(StatusCode::UNAUTHORIZED),
        Err(_) => {
            return axum::response::IntoResponse::into_response(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if auth_session.login(&user).await.is_err() {
        return axum::response::IntoResponse::into_response(StatusCode::INTERNAL_SERVER_ERROR);
    }

    axum::response::IntoResponse::into_response(Redirect::to("/voyager/webui"))
}
