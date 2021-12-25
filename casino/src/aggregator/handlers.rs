use std::convert::Infallible;

use hyper::{Body, Request, Response};

pub async fn handle_state(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

pub async fn handle_websocket(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

pub async fn handle_upload(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}
