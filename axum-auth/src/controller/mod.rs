pub mod auth;

use axum::{
    http::{header, StatusCode},
    response::Response,
};
use serde_json::json;

pub async fn hello_world() -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body("{\"message\": \"Hello, World!\"}".to_string())
        .unwrap_or_default()
}

pub async fn public_view_handler() -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "success": true,
                "data": {
                    "message": "This data is visible to all users"
                }
            })
            .to_string(),
        )
        .unwrap_or_default()
}
