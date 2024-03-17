use actix_web::HttpResponse;
use serde_json::json;

pub async fn hello_world() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": "Hello World!"
    }))
}
