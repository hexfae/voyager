use axum::http::StatusCode;

/// Returned when Voyager is a teapot (when
/// an unhandled HTTP request is sent).
pub async fn teapot() -> StatusCode {
    StatusCode::IM_A_TEAPOT
}
