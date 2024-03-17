use actix_web::{web::get, App, HttpServer};

use crate::controller::hello_world;

#[actix_web::main]
pub async fn setup() -> std::io::Result<()> {
    HttpServer::new(move || App::new().route("/", get().to(hello_world)))
        .workers(4)
        .bind("127.0.0.1:2424")
        .expect("Address should be free and valid")
        .run()
        .await
}
