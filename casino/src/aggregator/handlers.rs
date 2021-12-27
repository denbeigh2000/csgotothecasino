use std::convert::Infallible;

use futures_util::{SinkExt, Stream, StreamExt};
use hyper_tungstenite::hyper::{Body, Method, Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::{is_upgrade_request, HyperWebsocket};

use crate::csgofloat::{CsgoFloatClient, CsgoFloatFetchError};
use crate::steam::errors::MarketPriceFetchError;
use crate::steam::{MarketPriceClient, UnhydratedUnlock, Unlock};
use crate::store::{Store, StoreError};

use super::http::resp_400;
use super::websocket::{handle_emit, handle_recv};

#[derive(Debug)]
pub enum HandlerError {
    Transport(hyper::Error),
    Store(StoreError),
    MarketPrice(MarketPriceFetchError),
    CsgoFloat(CsgoFloatFetchError),
    Serde(serde_json::Error),
}

impl From<StoreError> for HandlerError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<MarketPriceFetchError> for HandlerError {
    fn from(e: MarketPriceFetchError) -> Self {
        Self::MarketPrice(e)
    }
}

impl From<CsgoFloatFetchError> for HandlerError {
    fn from(e: CsgoFloatFetchError) -> Self {
        Self::CsgoFloat(e)
    }
}

impl From<serde_json::Error> for HandlerError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl From<hyper::Error> for HandlerError {
    fn from(e: hyper::Error) -> Self {
        Self::Transport(e)
    }
}

pub struct Handler {
    store: Store,
    csgofloat_client: CsgoFloatClient,
    market_price_client: MarketPriceClient,
}

impl Handler {
    pub fn new(
        store: Store,
        csgofloat_client: CsgoFloatClient,
        market_price_client: MarketPriceClient,
    ) -> Self {
        Self {
            store,
            csgofloat_client,
            market_price_client,
        }
    }

    pub async fn save(&self, items: &[UnhydratedUnlock]) -> Result<(), HandlerError> {
        let urls: Vec<&str> = items.iter().map(|i| i.item_market_link.as_str()).collect();
        let float_info = self.csgofloat_client.get_bulk(&urls).await?;

        for item in items {
            let pricing = self.market_price_client.get(&item.item_market_name).await?;
            let hydrated = Unlock {
                key: item.key.clone(),
                case: item.case.clone(),
                item: float_info.get(&item.item_market_link).unwrap().clone(),

                item_value: pricing,
                at: item.at,
                name: item.name.clone(),
            };

            self.store.append_entry(item).await?;
            self.store.publish(&hydrated).await?;
        }

        Ok(())
    }

    pub async fn get_state(&self) -> Result<Vec<Unlock>, HandlerError> {
        let state = self.store.get_entries().await?;
        if state.is_empty() {
            return Ok(vec![]);
        }

        let urls: Vec<&str> = state.iter().map(|e| e.item_market_link.as_ref()).collect();

        let csgofloat_info = self.csgofloat_client.get_bulk(&urls).await?;
        let mut entries = Vec::with_capacity(state.len());
        for entry in state.into_iter() {
            let p = self
                .market_price_client
                .get(&entry.item_market_name)
                .await?;

            let f = csgofloat_info.get(&entry.item_market_link).unwrap().clone();

            entries.push(Unlock {
                key: entry.key,
                case: entry.case,
                item: f,
                item_value: p,

                at: entry.at,
                name: entry.name,
            });
        }

        Ok(entries)
    }

    pub async fn event_stream(&self) -> Result<impl Stream<Item = Unlock>, HandlerError> {
        let stream = self.store.get_event_stream().await?;

        Ok(stream)
    }
}

pub async fn handle_state(h: &Handler, req: Request<Body>) -> Result<Response<Body>, HandlerError> {
    if req.method() != Method::GET {
        return Ok(resp_400());
    }

    let state = h.get_state().await?;
    let state_data = serde_json::to_vec(&state)?;
    let resp = Response::builder().body(Body::from(state_data)).unwrap();

    Ok(resp)
}

pub async fn handle_upload(
    h: &Handler,
    mut req: Request<Body>,
) -> Result<Response<Body>, HandlerError> {
    if req.method() != Method::POST {
        eprintln!("bad request type");
        return Ok(resp_400());
    }

    let data = hyper::body::to_bytes(req.body_mut()).await?;
    let unlock: Vec<UnhydratedUnlock> = match serde_json::from_slice(&data) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("parsing failed: {}", e);
            return Ok(resp_400());
        }
    };

    let status = match h.save(&unlock).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            eprintln!("saving failed: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };

    let resp = Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap();

    Ok(resp)
}

pub async fn handle_websocket(
    h: &Handler,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    if !is_upgrade_request(&req) {
        return Ok(resp_400());
    }

    let (resp, socket) = hyper_tungstenite::upgrade(req, None).unwrap();
    let stream = h.event_stream().await.unwrap();
    tokio::spawn(handle_upgraded_websocket(Box::pin(stream), socket));

    Ok(resp)
}

async fn handle_upgraded_websocket<S: Stream<Item = Unlock> + Unpin>(
    mut stream: S,
    ws: HyperWebsocket,
) {
    let mut ws = ws.await.unwrap();
    loop {
        tokio::select! {
            msg = ws.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("error receiving message from websocket: {}", e);
                        eprintln!("closing connection");
                        return;
                    },
                    None => return,
                };

                if handle_recv(msg).await.unwrap() {
                    // This is a close message.
                    return;
                }
            },

            unlock = stream.next() => {
                match unlock {
                    Some(u) => {
                        handle_emit(&mut ws, u).await.unwrap();
                    },
                    None => {
                        // Server is closing, shutdown connection.
                        ws.send(Message::Close(None)).await.unwrap();
                        return
                    }
                }
            }
        }
    }
}
