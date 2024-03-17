use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

use crate::controller::auth::{get_token_handler, secret_view_handler};
use crate::controller::{hello_world, public_view_handler};

#[tokio::main(flavor = "multi_thread", worker_threads = 6)]
pub async fn startup() {
    let routes = Router::new()
        .route("/", get(hello_world))
        .route("/public-view", get(public_view_handler))
        .route("/get-token", post(get_token_handler))
        .route("/secret-view", get(secret_view_handler));

    let tcp_listener = TcpListener::bind("localhost:2323")
        .await
        .expect("Address should be free and valid");

    axum::serve(tcp_listener, routes)
        .await
        .expect("Error serving application");
}
