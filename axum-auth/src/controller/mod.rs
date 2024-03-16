use axum::{
    http::{header, StatusCode},
    response::Response,
};

pub async fn hello_world() -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{\"message\": \"Hello, World!\"}".to_string())
        .unwrap_or_default()
}
