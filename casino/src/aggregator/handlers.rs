use std::fmt::{self, Display};

use futures_util::{SinkExt, Stream, StreamExt};
use hyper::header::AUTHORIZATION;
use hyper_tungstenite::hyper::{Body, Method, Request, Response, StatusCode};
use hyper_tungstenite::tungstenite::{self, Message};
use hyper_tungstenite::{is_upgrade_request, HyperWebsocket};

use super::http::{resp_400, resp_403, resp_500};
use super::keystore::KeyStore;
use super::websocket::{handle_emit, handle_recv, MessageSendError};
use crate::csgofloat::{CsgoFloatClient, CsgoFloatFetchError};
use crate::steam::errors::MarketPriceFetchError;
use crate::steam::{MarketPriceClient, UnhydratedUnlock, Unlock};
use crate::store::{Error as StoreError, Store};

#[derive(Debug)]
pub enum HandlerError {
    GetState(GetStateError),
    SaveItems(SaveItemsError),
    StreamItems(StreamError),
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandlerError::GetState(e) => write!(f, "error serving get_state request: {}", e),
            HandlerError::SaveItems(e) => write!(f, "error serving save request: {}", e),
            HandlerError::StreamItems(e) => write!(f, "error serving stream request: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum HydrationError {
    CasePrice(MarketPriceFetchError),
    ItemPrice(MarketPriceFetchError),
    FloatInfo(CsgoFloatFetchError),
}

impl From<CsgoFloatFetchError> for HydrationError {
    fn from(e: CsgoFloatFetchError) -> Self {
        Self::FloatInfo(e)
    }
}

impl Display for HydrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CasePrice(e) => write!(f, "error fetching case price: {}", e),
            Self::ItemPrice(e) => write!(f, "error fetching item price: {}", e),
            Self::FloatInfo(e) => write!(f, "error fetching float information: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum SaveItemsError {
    BadKey,
    PassingMultipleUsers,
    HydratingItem(HydrationError),
    SavingItem(StoreError),
    PublishingItem(StoreError),
    Transport(hyper::Error),
}

impl From<hyper::Error> for SaveItemsError {
    fn from(e: hyper::Error) -> Self {
        Self::Transport(e)
    }
}

impl From<HydrationError> for SaveItemsError {
    fn from(e: HydrationError) -> Self {
        Self::HydratingItem(e)
    }
}

impl From<CsgoFloatFetchError> for SaveItemsError {
    fn from(e: CsgoFloatFetchError) -> Self {
        SaveItemsError::HydratingItem(HydrationError::FloatInfo(e))
    }
}

impl Display for SaveItemsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadKey => write!(f, "bad/missing pre-shared key"),
            Self::PassingMultipleUsers => write!(f, "data received for all users must be the same"),
            Self::HydratingItem(e) => write!(f, "error hydrating case item: {}", e),
            Self::SavingItem(e) => write!(f, "error persisting item: {}", e),
            Self::PublishingItem(e) => write!(f, "error publishing new item event: {}", e),
            Self::Transport(e) => write!(f, "error communicating with client: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum GetStateError {
    HydratingItem(HydrationError),
    FetchingItems(StoreError),
    SerializingItems(serde_json::Error),
}

impl From<serde_json::Error> for GetStateError {
    fn from(e: serde_json::Error) -> Self {
        GetStateError::SerializingItems(e)
    }
}

impl From<HydrationError> for GetStateError {
    fn from(e: HydrationError) -> Self {
        Self::HydratingItem(e)
    }
}

impl From<StoreError> for GetStateError {
    fn from(e: StoreError) -> Self {
        Self::FetchingItems(e)
    }
}

impl From<CsgoFloatFetchError> for GetStateError {
    fn from(e: CsgoFloatFetchError) -> Self {
        GetStateError::HydratingItem(HydrationError::FloatInfo(e))
    }
}

impl Display for GetStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GetStateError::HydratingItem(e) => write!(f, "error hydrating items: {}", e),
            GetStateError::FetchingItems(e) => write!(f, "error getting items from store: {}", e),
            GetStateError::SerializingItems(e) => write!(f, "error serialising items: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct StreamError(StoreError);

impl From<StoreError> for StreamError {
    fn from(e: StoreError) -> Self {
        StreamError(e)
    }
}

impl Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error getting data stream: {}", self.0)
    }
}

pub struct Handler {
    store: Store,
    key_store: KeyStore,
    csgofloat_client: CsgoFloatClient,
    market_price_client: MarketPriceClient,
}

impl Handler {
    pub fn new(
        store: Store,
        key_store: KeyStore,
        csgofloat_client: CsgoFloatClient,
        market_price_client: MarketPriceClient,
    ) -> Self {
        Self {
            store,
            key_store,
            csgofloat_client,
            market_price_client,
        }
    }

    pub async fn save(&self, key: &str, items: &[UnhydratedUnlock]) -> Result<(), SaveItemsError> {
        if items.is_empty() {
            return Ok(());
        }

        let item = items.get(0).unwrap();
        if !self.key_store.verify(&item.name, key).unwrap_or(false) {
            return Err(SaveItemsError::BadKey);
        }

        // ensure all entries are for the same person
        if !items.iter().all(|i| i.name == item.name) {
            return Err(SaveItemsError::PassingMultipleUsers);
        }

        let urls: Vec<&str> = items.iter().map(|i| i.item_market_link.as_str()).collect();
        let float_info = self.csgofloat_client.get_bulk(&urls).await?;

        for item in items {
            let item_value = self
                .market_price_client
                .get(&item.item_market_name)
                .await
                .map_err(HydrationError::ItemPrice)?;
            let case_value = self
                .market_price_client
                .get(item.case.get_name())
                .await
                .map_err(HydrationError::CasePrice)?;

            let hydrated = Unlock {
                key: item.key.clone(),
                case: item.case.clone(),
                case_value,
                item: float_info.get(&item.item_market_link).unwrap().clone(),

                item_value,
                at: item.at,
                name: item.name.clone(),
            };

            self.store
                .append_entry(item)
                .await
                .map_err(SaveItemsError::SavingItem)?;
            self.store
                .publish(&hydrated)
                .await
                .map_err(SaveItemsError::PublishingItem)?;
        }

        Ok(())
    }

    pub async fn get_state(&self) -> Result<Vec<Unlock>, GetStateError> {
        let state = self.store.get_entries().await?;
        if state.is_empty() {
            return Ok(vec![]);
        }

        let urls: Vec<&str> = state.iter().map(|e| e.item_market_link.as_ref()).collect();

        let csgofloat_info = self.csgofloat_client.get_bulk(&urls).await?;
        let mut entries = Vec::with_capacity(state.len());
        for entry in state.into_iter() {
            let item_value = self
                .market_price_client
                .get(&entry.item_market_name)
                .await
                .map_err(HydrationError::ItemPrice)?;
            let case_value = self
                .market_price_client
                .get(entry.case.get_name())
                .await
                .map_err(HydrationError::CasePrice)?;

            let f = csgofloat_info.get(&entry.item_market_link).unwrap().clone();

            entries.push(Unlock {
                key: entry.key,
                case: entry.case,
                case_value,
                item: f,
                item_value,

                at: entry.at,
                name: entry.name,
            });
        }

        Ok(entries)
    }

    pub async fn event_stream(&self) -> Result<impl Stream<Item = Unlock>, StreamError> {
        let stream = self.store.get_event_stream().await?;

        Ok(stream)
    }
}

pub async fn handle_state(
    h: &Handler,
    req: Request<Body>,
) -> Result<Response<Body>, GetStateError> {
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
) -> Result<Response<Body>, SaveItemsError> {
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

    let key = match req.headers().get(AUTHORIZATION) {
        Some(k) => k.to_str().unwrap(),
        None => return Ok(resp_403()),
    };

    let status = match h.save(key, &unlock).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            eprintln!("saving failed: {}", e);
            match e {
                SaveItemsError::BadKey | SaveItemsError::PassingMultipleUsers => {
                    StatusCode::UNAUTHORIZED
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
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
) -> Result<Response<Body>, StreamError> {
    if !is_upgrade_request(&req) {
        return Ok(resp_400());
    }

    let stream = match h.event_stream().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error opening event stream: {}", e);
            return Ok(resp_500());
        }
    };

    let (resp, socket) = hyper_tungstenite::upgrade(req, None).unwrap();
    tokio::spawn(spawn_handle_websocket(Box::pin(stream), socket));

    Ok(resp)
}

#[derive(Debug)]
enum WebsocketServingError {
    Upgrading(tungstenite::Error),
    Receiving(tungstenite::Error),
    Sending(tungstenite::Error),
}

impl Display for WebsocketServingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upgrading(e) => write!(f, "error upgrading: {}", e),
            Self::Receiving(e) => write!(f, "error receiving message: {}", e),
            Self::Sending(e) => write!(f, "error sending message: {}", e),
        }
    }
}

async fn spawn_handle_websocket<S: Stream<Item = Unlock> + Unpin>(stream: S, ws: HyperWebsocket) {
    if let Err(e) = handle_upgraded_websocket(stream, ws).await {
        eprintln!("error serving websocket: {}", e);
    }
}

async fn handle_upgraded_websocket<S: Stream<Item = Unlock> + Unpin>(
    mut stream: S,
    ws: HyperWebsocket,
) -> Result<(), WebsocketServingError> {
    let mut ws = ws.await.map_err(WebsocketServingError::Upgrading)?;

    loop {
        tokio::select! {
            msg = ws.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("error receiving message from websocket: {}", e);
                        eprintln!("closing connection");
                        return Err(WebsocketServingError::Receiving(e));
                    },
                    None => return Ok(()),
                };

                if handle_recv(msg) {
                    // Client sent a close message.
                    return Ok(());
                }
            },

            unlock = stream.next() => {
                let unlock = match unlock {
                    Some(u) => u,
                    None => {
                        // Server is closing, shutdown connection.
                        ws.send(Message::Close(None)).await.unwrap();
                        // TODO: Send termination info?
                        return Ok(());
                    },
                };

                handle_emit(&mut ws, unlock).await.or_else(|e| match e {
                    MessageSendError::Transport(e) => Err(WebsocketServingError::Sending(e)),
                    MessageSendError::Serde(e) => {
                        eprintln!("error marshaling message to send to client: {}", e);
                        Ok(())
                    },
                })?;
            }
        }
    }
}
