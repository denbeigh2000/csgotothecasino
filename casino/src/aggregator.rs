use std::convert::Infallible;
use std::time::Duration;

use bb8_redis::bb8::PooledConnection;
use bb8_redis::RedisConnectionManager;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Method, Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::{is_upgrade_request, HyperWebsocket, WebSocketStream};
use route_recognizer::Router;
use tokio::sync::watch::{Receiver, Sender};

use crate::steam::Unlock;
use crate::steam::{ItemDescription, TrivialItem};

lazy_static::lazy_static! {
    static ref ROUTER: Router<Route> = router();
}

#[cfg(feature = "stub")]
lazy_static::lazy_static! {
    static ref STUB_ITEM: ItemDescription = serde_json::from_str(r##"{
    "origin": 8,
    "quality": 12,
    "rarity": 3,
    "a": "24028753890",
    "d": "1030953410031234813",
    "paintseed": 435,
    "defindex": 19,
    "paintindex": 776,
    "stickers": [
      {
        "stickerId": 4965,
        "slot": 0,
        "codename": "stockh2021_team_navi_gold",
        "material": "stockh2021/navi_gold",
        "name": "Natus Vincere (Gold) | Stockholm 2021"
      },
      {
        "stickerId": 4981,
        "slot": 1,
        "codename": "stockh2021_team_g2_gold",
        "material": "stockh2021/g2_gold",
        "name": "G2 Esports (Gold) | Stockholm 2021"
      },
      {
        "stickerId": 1693,
        "slot": 2,
        "codename": "de_nuke_gold",
        "material": "tournament_assets/de_nuke_gold",
        "name": "Nuke (Gold)"
      },
      {
        "stickerId": 5053,
        "slot": 3,
        "codename": "stockh2021_team_pgl_gold",
        "material": "stockh2021/pgl_gold",
        "name": "PGL (Gold) | Stockholm 2021"
      }
    ],
    "floatid": "24028753890",
    "floatvalue": 0.11490528285503387,
    "s": "76561198035933253",
    "m": "0",
    "imageurl": "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_p90_hy_blueprint_aqua_light_large.35f86b3da01a31539d5a592958c96356f63d1675.png",
    "min": 0,
    "max": 0.5,
    "weapon_type": "P90",
    "item_name": "Facility Negative",
    "rarity_name": "Mil-Spec Grade",
    "quality_name": "Souvenir",
    "origin_name": "Found in Crate",
    "wear_name": "Minimal Wear",
    "full_item_name": "Souvenir P90 | Facility Negative (Minimal Wear)"
  }"##).unwrap();
}

enum Route {
    State,
    Stream,
}

fn router() -> Router<Route> {
    let mut router = Router::new();
    router.add("/", Route::State);
    router.add("/stream", Route::Stream);

    router
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
    events: Receiver<Unlock>,
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

fn resp_404() -> Response<Body> {
    Response::builder().status(404).body(Body::empty()).unwrap()
}

fn resp_400() -> Response<Body> {
    Response::builder().status(400).body(Body::empty()).unwrap()
}

#[cfg(not(feature = "stub"))]
async fn handle_state(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

#[cfg(not(feature = "stub"))]
async fn handle_websocket(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

#[cfg(feature = "stub")]
async fn handle_state(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::GET {
        return Ok(resp_400());
    }

    let data = vec![Unlock {
        name: "denbeigh".into(),
        item: STUB_ITEM.clone(),
        case: TrivialItem::new("Chroma Case".into(), None),
        key: Some(TrivialItem::new("Chroma Case Key".into(), None)),

        at: Utc::now(),
    }];

    let encoded_data = serde_json::to_vec(&data).unwrap();

    let resp = Response::builder()
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(encoded_data))
        .unwrap();

    Ok(resp)
}

#[cfg(feature = "stub")]
async fn handle_websocket(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if !is_upgrade_request(&req) {
        return Ok(resp_400());
    }

    let (resp, socket) = hyper_tungstenite::upgrade(req, None).unwrap();
    tokio::spawn(async move {
        let mut ws = socket.await.unwrap();
        let mut timer = tokio::time::interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                    msg = ws.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => return,
                    };

                    if handle_recv(msg.unwrap()).await.unwrap() {
                        return;
                    }
                }
                _ = timer.tick() => {
                    send_unlock(&mut ws).await;
                }
            }
        }
    });

    Ok(resp)
}

#[cfg(feature = "stub")]
async fn send_unlock(socket: &mut WebSocketStream<Upgraded>) {
    let unlock = Unlock {
        key: Some(TrivialItem::new("Chroma Case Key".into(), None)),
        case: TrivialItem::new("Chroma Case".into(), None),
        item: STUB_ITEM.clone(),

        at: Utc::now(),
        name: "denbeigh".into(),
    };

    handle_emit(socket, unlock).await.unwrap();
}

async fn handle_recv(msg: Message) -> Result<bool, Infallible> {
    Ok(match msg {
        Message::Close(_) => {
            eprintln!("received close, shutting down");
            true
        }
        _ => false,
    })
}

async fn handle_emit(
    socket: &mut WebSocketStream<Upgraded>,
    unlock: Unlock,
) -> Result<(), Infallible> {
    let encoded = serde_json::to_vec(&unlock).unwrap();
    let msg = Message::Binary(encoded);
    socket.send(msg).await.unwrap();

    Ok(())
}
