use hyper_tungstenite::hyper::{Body, Response};
use route_recognizer::Router;

pub fn resp_500() -> Response<Body> {
    Response::builder().status(500).body(Body::empty()).unwrap()
}

pub fn resp_404() -> Response<Body> {
    Response::builder().status(404).body(Body::empty()).unwrap()
}

pub fn resp_403() -> Response<Body> {
    Response::builder().status(403).body(Body::empty()).unwrap()
}

pub fn resp_400() -> Response<Body> {
    Response::builder().status(400).body(Body::empty()).unwrap()
}

pub enum Route {
    State,
    Stream,
    Upload,
}

pub fn router() -> Router<Route> {
    let mut router = Router::new();
    router.add("/", Route::State);
    router.add("/stream", Route::Stream);
    router.add("/upload", Route::Upload);

    router
}
