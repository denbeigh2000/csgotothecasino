use std::convert::Infallible;
use std::sync::Arc;

use futures_util::{Stream, SinkExt, StreamExt};
use hyper::{Body, Method, Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::{is_upgrade_request, HyperWebsocket, WebSocketStream};
use tokio::sync::watch::{self, Receiver, Sender};

use crate::csgofloat::CsgoFloatClient;
use crate::steam::{MarketPriceClient, UnhydratedUnlock, Unlock};
use crate::store::Store;

use super::http::resp_400;
use super::websocket::{handle_emit, handle_recv};

pub struct Handler {
    events_rx: Receiver<Option<UnhydratedUnlock>>,
    events_tx: Arc<Sender<Option<UnhydratedUnlock>>>,

    store: Store,
    csgofloat_client: CsgoFloatClient,
    market_price_client: MarketPriceClient,
}

pub fn new_handler_unimplemented() -> Handler {
    todo!()
}

impl Handler {
    pub fn new(
        store: Store,
        csgofloat_client: CsgoFloatClient,
        market_price_client: MarketPriceClient,
    ) -> Self {
        let (events_tx, events_rx) = watch::channel(None);
        let events_tx = Arc::new(events_tx);
        Self {
            events_rx,
            events_tx,
            store,
            csgofloat_client,
            market_price_client,
        }
    }

    pub async fn save(&self, items: &[UnhydratedUnlock]) -> Result<(), Infallible> {
        for item in items {
            let item = item.clone();
            self.store.append_entry(&item).await?;
            self.events_tx.send(Some(item)).unwrap();
        }

        Ok(())
    }

    pub async fn get_state(&self) -> Result<Vec<Unlock>, Infallible> {
        let state = self.store.get_entries().await?;
        let urls: Vec<&str> = state.iter().map(|e| e.item_market_link.as_ref()).collect();

        let mut csgofloat_info = self.csgofloat_client.get_bulk(&urls).await?;
        let mut entries = Vec::with_capacity(state.len());
        for entry in state.into_iter() {
            let p = self
                .market_price_client
                .get(&entry.item_market_name)
                .await
                .unwrap();
            let f = csgofloat_info.remove(&entry.item_market_link).unwrap();

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

    pub async fn event_stream(&self) -> Result<impl Stream<Item = Unlock>, Infallible> {
        let stream = self.store.get_event_stream().await.unwrap();

        Ok(stream)
    }
}

pub async fn handle_state(h: &Handler, req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::GET {
        return Ok(resp_400());
    }

    let state = h.get_state().await.unwrap();
    let state_data = serde_json::to_vec(&state).unwrap();
    let resp = Response::builder().body(Body::from(state_data)).unwrap();

    Ok(resp)
}

pub async fn handle_upload(
    h: &Handler,
    mut req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    if req.method() != Method::POST {
        eprintln!("bad request type");
        return Ok(resp_400());
    }

    let data = hyper::body::to_bytes(req.body_mut()).await.unwrap();
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
            eprintln!("saving failed: {}", e);
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
    let stream = h.event_stream().await?;
    tokio::spawn(handle_upgraded_websocket(Box::pin(stream), socket));

    Ok(resp)
}

async fn handle_upgraded_websocket<S: Stream<Item = Unlock> + Unpin>(mut stream: S, ws: HyperWebsocket) {
    let mut ws = ws.await.unwrap();
    loop {
        tokio::select! {
            msg = ws.next() => {
                let msg = match msg {
                    Some(m) => m,
                    None => return,
                };

                if handle_recv(msg.unwrap()).await.unwrap() {
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
