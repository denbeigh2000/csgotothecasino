use std::convert::Infallible;
use std::time::Duration;

use chrono::Utc;
use futures_util::StreamExt;
use hyper_tungstenite::hyper::header::CONTENT_TYPE;
use hyper_tungstenite::hyper::upgrade::Upgraded;
use hyper_tungstenite::hyper::{Body, Method, Request, Response};
use hyper_tungstenite::{is_upgrade_request, WebSocketStream};

use crate::aggregator::http::resp_400;
use crate::aggregator::websocket::{handle_emit, handle_recv};
use crate::steam::{ItemDescription, MarketPrices, RawMarketPrices, TrivialItem, Unlock};

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

    static ref STUB_ITEM_VALUE: RawMarketPrices = serde_json::from_str(r##"{
        "success": true,
        "lowest_price": "$1.81",
        "volume": "3",
        "median_price": "$1.68"
    }"##).unwrap();

    static ref STUB_CASE_VALUE: RawMarketPrices = serde_json::from_str(r##"{
        "success": true,
        "lowest_price": "$0.00",
        "volume": "300000",
        "median_price": "$0.00"
    }"##).unwrap();
}

static CLUTCH_CASE_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFY5naqQIz4R7Yjix9bZkvKiZrmAzzlTu5AoibiT8d_x21Wy8hY_MWz1doSLMlhpM3FKbNs";
static CLUTCH_CASE_KEY_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiev8ZQQ30KubIWVDudrgkNncw6-hY-2Fkz1S7JRz2erHodnzig2xqUVvYDrtZNjCAC7WDrU";

#[derive(Default)]
pub struct Handler {}

pub async fn handle_websocket(
    _h: &Handler,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
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

async fn send_unlock(socket: &mut WebSocketStream<Upgraded>) {
    let item_value: MarketPrices = (*STUB_ITEM_VALUE).clone().try_into().unwrap();
    let case_value: MarketPrices = (*STUB_CASE_VALUE).clone().try_into().unwrap();
    let unlock = Unlock {
        key: Some(TrivialItem::new(
            "Clutch Case Key",
            CLUTCH_CASE_KEY_IMG,
            None,
        )),
        case: TrivialItem::new("Clutch Case", CLUTCH_CASE_IMG, None),
        item: STUB_ITEM.clone(),
        item_value,
        case_value,

        at: Utc::now(),
        name: "denbeigh".into(),
    };

    handle_emit(socket, unlock).await.unwrap();
}

pub async fn handle_state(_h: &Handler, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::GET {
        return Ok(resp_400());
    }

    let item_value: MarketPrices = (*STUB_ITEM_VALUE).clone().try_into().unwrap();
    let case_value: MarketPrices = (*STUB_CASE_VALUE).clone().try_into().unwrap();
    let data = vec![Unlock {
        name: "denbeigh".into(),
        item: STUB_ITEM.clone(),
        item_value,
        case: TrivialItem::new("Clutch Case", CLUTCH_CASE_IMG, None),
        case_value,
        key: Some(TrivialItem::new(
            "Clutch Case Key",
            CLUTCH_CASE_KEY_IMG,
            None,
        )),

        at: Utc::now(),
    }];

    let encoded_data = serde_json::to_vec(&data).unwrap();

    let resp = Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(encoded_data))
        .unwrap();

    Ok(resp)
}

pub async fn handle_upload(
    _h: &Handler,
    _req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}
