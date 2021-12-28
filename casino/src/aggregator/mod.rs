use std::convert::Infallible;
use std::sync::Arc;

use hyper_tungstenite::hyper::server::conn::AddrStream;
use hyper_tungstenite::hyper::service::{make_service_fn, service_fn};
use hyper_tungstenite::hyper::{Body, Request, Response};
use route_recognizer::Router;

mod http;
pub mod keystore;
mod websocket;

#[cfg(feature = "not-stub")]
mod handlers;
#[cfg(feature = "not-stub")]
pub use crate::aggregator::handlers::Handler;
#[cfg(feature = "not-stub")]
use crate::aggregator::handlers::{handle_state, handle_upload, handle_websocket};

#[cfg(not(feature = "not-stub"))]
mod stub_handlers;
#[cfg(not(feature = "not-stub"))]
pub use crate::aggregator::stub_handlers::Handler;
#[cfg(not(feature = "not-stub"))]
use crate::aggregator::stub_handlers::{handle_state, handle_upload, handle_websocket};

use crate::aggregator::http::{resp_404, router, Route};

lazy_static::lazy_static! {
    static ref ROUTER: Router<Route> = router();
}

async fn ctrl_c() {
    tokio::signal::ctrl_c().await.unwrap();

    eprintln!("shutting down");
}

pub async fn serve(handler: Handler) -> Result<(), Infallible> {
    let h = Arc::new(handler);

    let svc = make_service_fn(move |_socket: &AddrStream| {
        let h = h.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let h = Arc::clone(&h);
                async move {
                    let resp: Result<Response<Body>, Infallible> = handle_request(&*h, req).await;

                    resp
                }
            }))
        }
    });

    let addr = "0.0.0.0:7000".parse().unwrap();
    hyper::Server::bind(&addr)
        .serve(svc)
        .with_graceful_shutdown(ctrl_c())
        .await
        .unwrap();

    Ok(())
}

async fn handle_request(h: &Handler, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match ROUTER.recognize(req.uri().path()) {
        Ok(m) => Ok(match m.handler() {
            Route::State => handle_state(h, req).await.unwrap(),
            Route::Stream => handle_websocket(h, req).await.unwrap(),
            Route::Upload => handle_upload(h, req).await.unwrap(),
        }),
        Err(_) => Ok(resp_404()),
    }
}
