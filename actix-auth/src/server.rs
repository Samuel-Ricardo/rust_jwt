use actix_web::{
    web::{get, post},
    App, HttpServer,
};

use crate::controller::{
    auth::{get_token_handler, secret_view_handler},
    hello_world, public_view_handler,
};

#[actix_web::main]
pub async fn setup() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .route("/", get().to(hello_world))
            .route("/public-view", get().to(public_view_handler))
            .route("/get-token", post().to(get_token_handler))
            .route("/secret-view", get().to(secret_view_handler))
    })
    .workers(4)
    .bind("127.0.0.1:8080")
    .expect("Address should be free and valid")
    .run()
    .await
}
