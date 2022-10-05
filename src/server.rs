use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio;

pub async fn http_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(root_path))
        .route("/healthcheck", get(healthcheck));
    println!("Starting server!");
    axum::Server::bind(&addr)
        .http1_only(false)
        .http2_only(false)
        .tcp_nodelay(true)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn root_path() -> &'static str {
    "you have reached the root path. Good for you"
}

async fn healthcheck() -> &'static str {
    "Healthy!"
}

pub fn server_runtime(addr: SocketAddr) {
    println!("Starting server runtime");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(2)
        .enable_io()
        .thread_name("http server")
        .build()
        .expect("Couldn't build server");
    runtime.block_on(http_server(addr)).expect("Server failed");
}
