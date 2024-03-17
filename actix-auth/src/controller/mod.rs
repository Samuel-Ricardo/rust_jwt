use actix_web::HttpResponse;
use serde_json::json;

pub async fn hello_world() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": "Hello World!"
    }))
}

pub async fn public_view_handler() -> HttpResponse {
    HttpResponse::Ok().json(json!({
      "success": true,
      "data": {
        "message": "This data is visible to all users"
      }
    }))
}
