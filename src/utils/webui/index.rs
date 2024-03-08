use askama_axum::{IntoResponse, Template};
use axum::{extract::State, response::Html};

use crate::prelude::*;

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    username: String,
    levels: Vec<Parsed>,
}

pub async fn index(
    auth_session: AuthSession,
    State(db): State<SharedAppState>,
) -> impl IntoResponse {
    let levels = db.keys_and_parsed_levels();
    auth_session
        .user
        .map_or(Html(r"unauthorized").into_response(), |user| {
            Index {
                username: user.username,
                levels,
            }
            .into_response()
        })
}
