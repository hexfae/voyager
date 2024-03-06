use axum::{extract::State, http::StatusCode, response::Html};

use crate::prelude::*;

pub async fn index(
    auth_session: AuthSession,
    State(db): State<SharedAppState>,
) -> Html<&'static str> {
    auth_session
        .user
        .map_or(Html(r"unauthorized"), |user| Html(r"hello"))
}
