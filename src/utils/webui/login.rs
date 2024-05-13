//! Log in.

use crate::prelude::*;

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};

/// The login page.
pub async fn get() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/voyager/webui/login" method="post">
                    <label for="username">
                        username:
                        <input type="text" name="username">
                    </label>

                    <label>
                        password:
                        <input type="password" name="password">
                    </label>

                    <input type="submit" value="submit">
                </form>
            </body>
        </html>
        "#,
    )
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
