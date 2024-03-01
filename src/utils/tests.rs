use std::str::FromStr;

use crate::{parser::Level, routers, server::AppState};
use axum::{
    routing::{any, delete, get, post, put},
    Router,
};
use axum_test::TestServer;
use ulid::Ulid;

fn new_app() -> Router {
    let levels = AppState::new();
    // levels.insert(
    //     Ulid::from_str("01HQNDEW9C7TV1RCMQZAJV318V").expect("valid key"),
    //     Level::from("1|V2FsbGtpY2s=|VGhlIGZpcnN0IGxldmVsIHRvIGJlIHVwbG9hZGVkIHRvIHRoZSBzZXJ2ZXJzIQ==|bXNjX2JlZWNpcmNsZQ==|QW5vbnltb3Vz|2685020332|2|flexwa16wa04X1wa17ptX3flptX1st00flX2ptX5flX2ptflX7ptflX3ptX1wa10wa14ptflX3ptflX4ptwa03wa17flX4ptflX3ptX1wa06flX5ptX1flX2ptflwa06flX4ptX3flptX1wa13wa09wa10X11wa11|emX9cgemX15tnemgocc1emplemX20cl0emX15cf1emX10lvemcf1moemX6csemX30|20240227|20240227"),
    // );
    // levels.insert(
    //     Ulid::from_str("01HQNE7J0ZKY8KT1WK0EMCKBB7").expect("valid key"),
    //     Level::from("1|V2FsbGtpY2s=|VGhlIGZpcnN0IGxldmVsIHRvIGJlIHVwbG9hZGVkIHRvIHRoZSBzZXJ2ZXJzIQ==|bXNjX2JlZWNpcmNsZQ==|QW5vbnltb3Vz|2685020332|2|flexwa16wa04X1wa17ptX3flptX1st00flX2ptX5flX2ptflX7ptflX3ptX1wa10wa14ptflX3ptflX4ptwa03wa17flX4ptflX3ptX1wa06flX5ptX1flX2ptflwa06flX4ptX3flptX1wa13wa09wa10X11wa11|emX9cgemX15tnemgocc1emplemX20cl0emX15cf1emX10lvemcf1moemX6csemX30|20240227|20240227"),
    // );
    // levels.insert(
    //     Ulid::from_str("01HQNE88QJTEHHAW9ZFREQ5W5A").expect("valid key"),
    //     Level::from("1|V2FsbGtpY2s=|VGhlIGZpcnN0IGxldmVsIHRvIGJlIHVwbG9hZGVkIHRvIHRoZSBzZXJ2ZXJzIQ==|bXNjX2JlZWNpcmNsZQ==|QW5vbnltb3Vz|2685020332|2|flexwa16wa04X1wa17ptX3flptX1st00flX2ptX5flX2ptflX7ptflX3ptX1wa10wa14ptflX3ptflX4ptwa03wa17flX4ptflX3ptX1wa06flX5ptX1flX2ptflwa06flX4ptX3flptX1wa13wa09wa10X11wa11|emX9cgemX15tnemgocc1emplemX20cl0emX15cf1emX10lvemcf1moemX6csemX30|20240227|20240227"),
    // );
    Router::new()
        .route("/voyager", get(routers::get::get))
        .route("/voyager/:key", get(routers::get::levels_exist))
        .route("/voyager", post(routers::post::post))
        .route("/voyager", put(routers::put::put))
        .route("/voyager", delete(routers::delete::delete))
        .route("/voyager", any(routers::teapot::teapot))
        .with_state(levels)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    use axum_test::TestServerConfig;

    let app = new_app();
    let config = TestServerConfig::builder()
        // .save_cookies()
        // .expect_success_by_default()
        // .mock_transport()
        .build();

    TestServer::new_with_config(app, config).expect("could not start test server")
}

#[cfg(test)]
mod voyager_tests {
    use crate::tests::new_test_app;
    use anyhow::Result;
    use axum::http::StatusCode;
    use pretty_assertions::assert_eq;
    use ulid::Ulid;

    #[tokio::test]
    async fn test_get() -> Result<()> {
        // TODO: this
        let server = new_test_app();
        println!("hi");
        let response = server.get("/voyager").await;
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
