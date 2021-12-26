use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;

use hyper_tungstenite::hyper::server::conn::AddrStream;
use hyper_tungstenite::hyper::service::{make_service_fn, service_fn};
use hyper_tungstenite::hyper::{Body, Request, Response};
use route_recognizer::Router;

mod http;
mod websocket;

#[cfg(feature = "not-stub")]
mod handlers;
#[cfg(feature = "not-stub")]
use crate::aggregator::handlers::{handle_state, handle_upload, handle_websocket};
#[cfg(feature = "not-stub")]
pub use crate::aggregator::handlers::{new_handler_unimplemented, Handler};

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

pub async fn serve<F, Fut>(make_handler: F) -> Result<(), Infallible>
where
    Fut: Future<Output = Handler> + Send + 'static,
    F: Fn() -> Fut + Copy + Send + Sync + 'static,
{
    let svc = make_service_fn(|_socket: &AddrStream| async move {
        let h = make_handler().await;
        let h = Arc::new(h);

        Ok::<_, Infallible>(service_fn(move |req| {
            let h = Arc::clone(&h);
            async move {
                let resp: Result<Response<Body>, Infallible> = handle_request(&*h, req).await;

                resp
            }
        }))
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
        Ok(m) => match m.handler() {
            Route::State => handle_state(h, req).await,
            Route::Stream => handle_websocket(h, req).await,
            Route::Upload => handle_upload(h, req).await,
        },
        Err(_) => Ok(resp_404()),
    }
}