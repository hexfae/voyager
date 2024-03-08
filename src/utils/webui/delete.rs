use crate::prelude::*;
use axum::{
    extract::{Path, State},
    response::Redirect,
};

pub async fn delete(State(db): State<SharedAppState>, key: Path<String>) -> Redirect {
    let _ = db.delete(key.as_str());
    Redirect::to("/voyager/webui")
}
