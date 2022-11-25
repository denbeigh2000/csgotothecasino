use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing;
use hyper_tungstenite::hyper::server::conn::AddrStream;
use hyper_tungstenite::hyper::service::{make_service_fn, service_fn};
use hyper_tungstenite::hyper::{Body, Request, Response};
use route_recognizer::Router;

mod http;
pub mod keystore;
mod websocket;

mod handlers;
pub use self::handlers::Handler;
use self::handlers::{handle_state, handle_state_hyper, handle_upload, handle_upload_hyper, handle_websocket, HandlerError};
use self::http::{resp_404, resp_500, router, Route};

lazy_static::lazy_static! {
    static ref ROUTER: Router<Route> = router();
}

async fn ctrl_c() {
    tokio::signal::ctrl_c().await.unwrap();

    log::info!("shutting down");
}

pub async fn serve(bind_addr: &SocketAddr, handler: Handler) -> Result<(), hyper::Error> {
    let handler = Arc::new(handler);
    let app = routing::Router::new()
        .route("/", routing::get(handle_state))
        .route("/upload", routing::post(handle_upload))
        .route("/stream", routing::get(handle_websocket))
        .with_state(handler);

    axum::Server::bind(bind_addr).serve(app.into_make_service()).await
}

pub async fn serve_hyper(bind_addr: &SocketAddr, handler: Handler) -> Result<(), Infallible> {
    let h = Arc::new(handler);

    let svc = make_service_fn(move |_socket: &AddrStream| {
        let h = h.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let h = Arc::clone(&h);
                async move {
                    let resp: Result<Response<Body>, Infallible> = handle_request(&h, req).await;

                    resp
                }
            }))
        }
    });

    log::info!("serving on {}", bind_addr);

    hyper::Server::bind(bind_addr)
        .serve(svc)
        .with_graceful_shutdown(ctrl_c())
        .await
        .unwrap();

    Ok(())
}

async fn handle_request(h: &Handler, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match ROUTER.recognize(req.uri().path()) {
        Ok(m) => {
            let resp = match m.handler() {
                Route::State => handle_state_hyper(h, req).await.map_err(HandlerError::GetState),
                Route::Upload => handle_upload_hyper(h, req).await.map_err(HandlerError::SaveItems),
                _ => unimplemented!(),
            }
            .unwrap_or_else(|e| {
                log::error!("error serving request: {}", e);
                resp_500()
            });

            Ok(resp)
        }
        Err(_) => Ok(resp_404()),
    }
}
