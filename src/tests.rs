use crate::server::AppState;
use axum::{
    routing::{any, get, post},
    Router,
};
use axum_test::TestServer;

fn new_app() -> Router {
    let level = include_str!("serialized.lvl");
    let levels = AppState::from(level).expect("valid serialized database");
    Router::new()
        .route("/void_stranger", get(crate::routers::get::get))
        .route("/void_stranger", post(crate::routers::post::post))
        // .route("/void_stranger", put(routers::put::put))
        .route("/void_stranger", any(crate::routers::teapot::teapot))
        .with_state(levels)
}

#[cfg(test)]
fn new_test_app() -> TestServer {
    use axum_test::TestServerConfig;

    let app = new_app();
    let config = TestServerConfig::builder()
        .save_cookies()
        .expect_success_by_default()
        .mock_transport()
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
    async fn test_post() -> Result<()> {
        let server = new_test_app();
        let level = include_str!("normal.vsl");
        let response = server.post("/void_stranger").text(level).await;
        assert_eq!(response.status_code(), StatusCode::CREATED);
        let text = response.text();
        let key = Ulid::from_string(&text);
        assert!(key.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_get() -> Result<()> {
        // TODO: this
        let server = new_test_app();
        let _level = include_str!("normal.vsl");
        let response = server.get("/void_stranger").await;
        let text = response.text();
        let key = Ulid::from_string(&text);
        assert_eq!(response.status_code(), StatusCode::OK);
        assert!(key.is_ok());
        Ok(())
    }
}
