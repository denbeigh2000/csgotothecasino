use std::convert::Infallible;

use bb8_redis::bb8::PooledConnection;
use bb8_redis::RedisConnectionManager;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response};
use route_recognizer::Router;
use tokio::sync::watch::Receiver;

use crate::steam::UnhydratedUnlock;

mod http;
mod websocket;

#[cfg(feature = "not-stub")]
mod handlers;
#[cfg(feature = "not-stub")]
use crate::aggregator::handlers::*;

#[cfg(not(feature = "not-stub"))]
mod stub_handlers;
#[cfg(not(feature = "not-stub"))]
use crate::aggregator::stub_handlers::*;

use crate::aggregator::http::{router, Route, resp_404};

lazy_static::lazy_static! {
    static ref ROUTER: Router<Route> = router();
}

pub async fn serve() -> Result<(), Infallible> {
    let svc = make_service_fn(|_socket: &AddrStream| async move {
        Ok::<_, Infallible>(service_fn(move |req| async {
            let resp: Result<Response<Body>, Infallible> = handle_request(req).await;

            resp
        }))
    });

    let addr = "0.0.0.0:7000".parse().unwrap();
    hyper::Server::bind(&addr).serve(svc).await.unwrap();

    Ok(())
}

struct Handle<'a> {
    events: Receiver<UnhydratedUnlock>,
    conn: PooledConnection<'a, RedisConnectionManager>,
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match ROUTER.recognize(req.uri().path()) {
        Ok(m) => match m.handler() {
            Route::State => handle_state(req).await,
            Route::Stream => handle_websocket(req).await,
        },
        Err(_) => Ok(resp_404()),
    }
}
