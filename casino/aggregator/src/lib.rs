use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing;
use thiserror::Error;

pub mod keystore;
mod websocket;

mod handlers;
pub use self::handlers::Handler;
use self::handlers::{handle_state, handle_upload, handle_websocket};

async fn ctrl_c() {
    tokio::signal::ctrl_c().await.unwrap();

    log::info!("shutting down");
}

#[derive(Debug, Error)]
#[error("Failed to serve http: {0}")]
pub struct ServingError(#[from] hyper::Error);

pub async fn serve(bind_addr: &SocketAddr, handler: Handler) -> Result<(), ServingError> {
    let handler = Arc::new(handler);
    let app = routing::Router::new()
        .route("/", routing::get(handle_state))
        .route("/upload", routing::post(handle_upload))
        .route("/stream", routing::get(handle_websocket))
        .with_state(handler);

    axum::Server::bind(bind_addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(ctrl_c())
        .await?;

    Ok(())
}
