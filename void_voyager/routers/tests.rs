use crate::prelude::*;

use crate::utils::{
    routers,
    server::{AppState, VoyagerConfig, VoyagerData},
};
use axum::{
    routing::{any, delete, get, post, put},
    Router,
};
use axum_test::TestServer;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use void_codex::Level;

const VALID_LEVEL: &str = "1|V2FsbGtpY2s=|VGhlIGZpcnN0IHJlYWwgcHV6emxlIHRvIGJlIHVwbG9hZGVkIHRvIHRoZSBzZXJ2ZXJzISBUaXRsZSBpcyBhIGhpbnQuLi4=|bXNjX2JlZWNpcmNsZQ==|U2tpcmxleg==|2693408940|20240314|20240316|2|flexwa16wa04X2wa17ptX4flptX2st00flX3ptX6flX3ptflX8ptflX4ptX2wa10wa14ptflX4ptflX5ptwa03wa17flX5ptflX4ptX2wa06flX6ptX2flX3ptflwa06flX5ptX4flptX2wa13wa09wa10X12wa11|emX10cgemX16tnemgocc1emplemX21csemX16cf1emX11lvemcf1moemX7csemX31";

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1));

impl AppState {
    pub fn for_tests() -> SharedAppState {
        let data = VoyagerData::for_tests();
        let config = RwLock::new(VoyagerConfig::default());
        Arc::new(Self { data, config })
    }
}

impl VoyagerData {
    fn for_tests() -> Self {
        let config = VoyagerConfig::default();
        let latest_version = config.latest_format_version.0;
        let allowed_songs = config.allowed_songs.0;
        let key = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("valid key");
        let level = Level::new(VALID_LEVEL, LOCALHOST)
            .into_parsed(latest_version, allowed_songs)
            .expect("valid level")
            .into_level();
        Self {
            levels: DashMap::from_iter([(key, level)]),
            ..Default::default()
        }
    }
}

#[cfg(test)]
pub fn new_app() -> Router {
    let db = AppState::for_tests();
    Router::new()
        .route("/voyager", get(routers::get::get))
        .route("/voyager/:keys", get(routers::get::levels_exist))
        .route("/voyager/version", get(routers::version::version))
        .route("/voyager", post(routers::post::post))
        .route("/voyager/orphanage", post(routers::post::orphanage))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
        .with_state(db)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    use axum_test::TestServerConfig;

    let app = new_app();
    let config = TestServerConfig::builder()
        .save_cookies()
        // .expect_success_by_default()
        .mock_transport()
        .build();

    TestServer::new_with_config(app, config).expect("could not start test server")
}

#[cfg(test)]
mod voyager_tests {
    use crate::prelude::*;
    use crate::utils::routers::tests::new_test_app;
    use axum::http::StatusCode;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_get() -> Result<()> {
        let server = new_test_app();
        let response = server.get("/voyager").await;
        dbg!(&response);
        assert_eq!(response.status_code(), StatusCode::OK);
        Ok(())
    }

    // #[tokio::test]
    // async fn test_post() -> Result<()> {
    //     let server = new_test_app();
    //     let level = include_str!("normal.vsl");
    //     let response = server.post("/void_stranger").text(level).await;
    //     assert_eq!(response.status_code(), StatusCode::CREATED);
    //     let text = response.text();
    //     let key = Ulid::from_string(&text);
    //     assert!(key.is_ok());
    //     Ok(())
    // }
}
