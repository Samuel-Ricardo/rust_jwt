use actix_web::{web::Json, HttpResponse};
use jwt_lib::model::user::User;
use serde_json::json;

use crate::middleware::auth::Auth;

pub async fn get_token_handler(Json(user): Json<User>) -> HttpResponse {
    let token = jwt_lib::get_jwt_secret(user);

    match token {
        Ok(token) => HttpResponse::Ok().json(json!({
          "success": true,
          "data": {
            "token": token
          }
        })),

        Err(error) => HttpResponse::BadRequest().json(json!({
          "success": false,
          "data": {
            "message": error
          }
        })),
    }
}

pub async fn secret_view_handler(Auth(user): Auth) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": user
    }))
}
