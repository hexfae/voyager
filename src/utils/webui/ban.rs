use crate::prelude::*;
use axum::{
    extract::{Path, State},
    response::Redirect,
};

pub async fn ban(State(db): State<SharedAppState>, ip: Path<String>) -> Redirect {
    let _ = db.ban(&ip);
    Redirect::to("/voyager/webui")
}
