use axum::{routing::get, Router};
use tokio::net::TcpListener;

use crate::controller::{hello_world, public_view_handler};

#[tokio::main(flavor = "multi_thread", worker_threads = 6)]
pub async fn startup() {
    let routes = Router::new()
        .route("/", get(hello_world))
        .route("/public-view", get(public_view_handler));

    let tcp_listener = TcpListener::bind("127.0.0.1:2323")
        .await
        .expect("Address should be free and valid");

    axum::serve(tcp_listener, routes)
        .await
        .expect("Error serving application");
}
