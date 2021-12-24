use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8::PooledConnection;
use bb8_redis::redis::aio::Connection;
use hyper::{Body, Request, Response};
use hyper::service::{make_service_fn, service_fn};
use hyper::server::conn::AddrStream;
use tokio::sync::watch::{Sender, Receiver};

use std::convert::Infallible;

use crate::steam::Unlock;

pub async fn serve() -> Result<(), Infallible> {
    let svc = make_service_fn(|_socket: &AddrStream| {
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                async {
                    let resp: Result<Response<Body>, Infallible> = handle_request(req).await;

                    resp
                }
            }))
        }
    });

    Ok(())
}

struct Handle<'a> {
    events: Receiver<Unlock>,
    conn: PooledConnection<'a, RedisConnectionManager>,
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

async fn get_state(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    todo!()
}
