use crate::prelude::*;

use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn get(Query(NextUrl { next }): Query<NextUrl>) -> Html<&'static str> {
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
                        <input type="text" name="password">
                    </label>

                    <input type="submit" value="submit">
                </form>
            </body>
        </html>
        "#,
    )
}

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

    creds
        .next
        .map_or_else(
            || Redirect::to("/voyager/webui"),
            |next| Redirect::to(&next),
        )
        .into_response()
}
