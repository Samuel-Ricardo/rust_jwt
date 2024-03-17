use axum::{
    http::{header, StatusCode},
    response::Response,
    Json,
};
use jwt_lib::model::user::User;
use serde_json::json;

use crate::middleware::auth::Auth;

pub async fn get_token_handler(Json(user): Json<User>) -> Response<String> {
    let token = jwt_lib::get_jwt_secret(user);

    match token {
        Ok(token) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(
                json!({
                    "success": true,
                    "data":{
                        "token": token
                }
                })
                .to_string(),
            )
            .unwrap_or_default(),

        Err(error) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(header::CONTENT_TYPE, "application/json")
            .body(
                json!({
                    "success": false,
                    "data": {
                    "message": error
                }
                })
                .to_string(),
            )
            .unwrap_or_default(),
    }
}

pub async fn secret_view_handler(Auth(user): Auth) -> Response<String> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "success": true,
                "data": user
            })
            .to_string(),
        )
        .unwrap_or_default()
}
