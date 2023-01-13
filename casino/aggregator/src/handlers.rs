use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, TypedHeader};
use futures_util::{Stream, StreamExt};
use headers::authorization::Bearer;
use thiserror::Error;

use super::keystore::KeyStore;
use super::websocket::{handle_emit, handle_recv, MessageSendError};
use csgofloat::{CsgoFloatClient, CsgoFloatFetchError};
use steam::errors::MarketPriceFetchError;
use steam::{CountdownRequest, MarketPriceClient, UnhydratedUnlock, Unlock};
use store::{Store, StoreError};

const UNLOCK_EVENT_KEY: &str = "new_events";
const SYNC_EVENT_KEY: &str = "new_sync_events";

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("error serving get_state request: {0}")]
    GetState(#[from] GetStateError),
    #[error("error serving save request: {0}")]
    SaveItems(#[from] SaveItemsError),
    #[error("error serving stream request: {0}")]
    StreamItems(#[from] StreamError),
}

#[derive(Debug, Error)]
pub enum HydrationError {
    #[error("error fetching case price: {0}")]
    CasePrice(MarketPriceFetchError),
    #[error("error fetching item price: {0}")]
    ItemPrice(MarketPriceFetchError),
    #[error("error fetching float information: {0}")]
    FloatInfo(#[from] CsgoFloatFetchError),
}

#[derive(Debug, Error)]
pub enum SaveItemsError {
    #[error("bad/missing pre-shared key")]
    BadKey,
    #[error("error hydrating case item: {0}")]
    HydratingItem(#[from] HydrationError),
    #[error("error persisting item: {0}")]
    SavingItem(StoreError),
    #[error("error publishing new item event: {0}")]
    PublishingItem(StoreError),
    #[error("error communicating with client: {0}")]
    Transport(#[from] hyper::Error),
}

impl IntoResponse for SaveItemsError {
    fn into_response(self) -> Response {
        let status = match self {
            SaveItemsError::BadKey => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // TODO: Consider if this should be empty, if we should log something,
        // and if we should do any sort of logging/error reporting at a higher
        // level.
        (status, self.to_string()).into_response()
    }
}

impl From<CsgoFloatFetchError> for SaveItemsError {
    fn from(e: CsgoFloatFetchError) -> Self {
        SaveItemsError::HydratingItem(HydrationError::FloatInfo(e))
    }
}

#[derive(Debug, Error)]
pub enum GetStateError {
    #[error("error hydrating items: {0}")]
    HydratingItem(#[from] HydrationError),
    #[error("error getting items from store: {0}")]
    FetchingItems(#[from] StoreError),
    #[error("error serialising items: {0}")]
    SerializingItems(#[from] serde_json::Error),
}

impl IntoResponse for GetStateError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

impl From<CsgoFloatFetchError> for GetStateError {
    fn from(e: CsgoFloatFetchError) -> Self {
        GetStateError::HydratingItem(HydrationError::FloatInfo(e))
    }
}

#[derive(Debug, Error)]
#[error("error getting data stream: {0}")]
pub struct StreamError(#[from] StoreError);

impl IntoResponse for StreamError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
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

    pub async fn save(
        &self,
        key: &str,
        items: Vec<UnhydratedUnlock>,
    ) -> Result<(), SaveItemsError> {
        if items.is_empty() {
            return Ok(());
        }

        let name = self.key_store.get_user(key).ok_or(SaveItemsError::BadKey)?;
        let items = items
            .into_iter()
            .map(|u| UnhydratedUnlock {
                name: name.clone(),
                ..u
            })
            .collect::<Vec<_>>();

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
                .append_entry(&item)
                .await
                .map_err(SaveItemsError::SavingItem)?;
            self.store
                .publish(UNLOCK_EVENT_KEY, &hydrated)
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
        let stream = self.store.get_event_stream(UNLOCK_EVENT_KEY).await?;

        Ok(stream)
    }

    pub async fn sync_event_stream(
        &self,
    ) -> Result<impl Stream<Item = CountdownRequest>, StreamError> {
        let stream = self.store.get_event_stream(SYNC_EVENT_KEY).await?;

        Ok(stream)
    }
}

pub async fn handle_state(
    State(state): State<Arc<Handler>>,
) -> Result<Json<Vec<Unlock>>, GetStateError> {
    state.get_state().await.map(Json::from)
}

pub async fn handle_upload(
    State(state): State<Arc<Handler>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(body): Json<Vec<UnhydratedUnlock>>,
) -> Result<(), SaveItemsError> {
    let key = auth.0.token();
    state.save(key, body).await
}

pub async fn handle_countdown_request(
    State(state): State<Arc<Handler>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(body): Json<steam::CountdownRequest>,
) -> Result<(), SaveItemsError> {
    let key = auth.0.token();
    let name = state
        .key_store
        .get_user(key)
        .ok_or(SaveItemsError::BadKey)?;

    if name != "badcop_" {
        return Err(SaveItemsError::BadKey);
    }
    state
        .store
        .publish(SYNC_EVENT_KEY, &body)
        .await
        .map_err(SaveItemsError::PublishingItem)?;
    Ok(())
}

pub async fn handle_websocket(
    State(state): State<Arc<Handler>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        if let Ok(stream) = state.event_stream().await.map(Box::pin) {
            if let Err(e) = handle_upgraded_websocket(stream, socket).await {
                log::error!("error serving websocket: {e}");
            }
        }
    })
}

pub async fn handle_sync_websocket(
    State(state): State<Arc<Handler>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        if let Ok(stream) = state.sync_event_stream().await.map(Box::pin) {
            if let Err(e) = handle_upgraded_websocket(stream, socket).await {
                log::error!("error serving websocket: {e}");
            }
        }
    })
}

#[derive(Debug, Error)]
enum WebsocketServingError {
    #[error("error receiving message: {0}")]
    Receiving(axum::Error),
    #[error("error sending message: {0}")]
    Sending(axum::Error),
}

async fn handle_upgraded_websocket<T: serde::Serialize, S: Stream<Item = T> + Unpin>(
    mut stream: S,
    mut ws: WebSocket,
) -> Result<(), WebsocketServingError> {
    // let mut ws = ws.map_err(WebsocketServingError::Upgrading)?;

    loop {
        tokio::select! {
            msg = ws.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        log::warn!("error receiving message from websocket: {}", e);
                        log::warn!("closing connection");
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
                        log::error!("error marshaling message to send to client: {}", e);
                        Ok(())
                    },
                })?;
            }
        }
    }
}
