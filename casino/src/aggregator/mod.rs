use std::convert::Infallible;
use std::sync::Arc;

use hyper_tungstenite::hyper::server::conn::AddrStream;
use hyper_tungstenite::hyper::service::{make_service_fn, service_fn};
use hyper_tungstenite::hyper::{Body, Request, Response};
use route_recognizer::Router;

mod http;
pub mod keystore;
mod websocket;

mod handlers;
pub use crate::aggregator::handlers::Handler;
use crate::aggregator::handlers::{handle_state, handle_upload, handle_websocket};

use crate::aggregator::handlers::HandlerError;
use crate::aggregator::http::{resp_404, resp_500, router, Route};

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
        Ok(m) => {
            let resp = match m.handler() {
                Route::State => handle_state(h, req).await.map_err(HandlerError::GetState),
                Route::Stream => handle_websocket(h, req)
                    .await
                    .map_err(HandlerError::StreamItems),
                Route::Upload => handle_upload(h, req).await.map_err(HandlerError::SaveItems),
            }
            .unwrap_or_else(|e| {
                eprintln!("error serving request: {:?}", e);
                resp_500()
            });

            Ok(resp)
        }
        Err(_) => Ok(resp_404()),
    }
}
