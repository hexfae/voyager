//! Delete a level.

use crate::prelude::*;
use axum::{
    extract::{Path, State},
    response::Redirect,
};

/// Deletes a level from the database, if found.
pub async fn delete(State(db): State<SharedAppState>, key: Path<String>) -> Redirect {
    if let Ok(key) = &key.parse() {
        let _ = db.delete(key);
    }
    Redirect::to("/voyager/webui")
}
