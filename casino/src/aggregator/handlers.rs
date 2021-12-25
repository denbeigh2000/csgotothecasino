use std::convert::Infallible;
use std::sync::Arc;

use futures_util::Stream;
use hyper::{Body, Request, Response};
use tokio::sync::watch::{self, Receiver, Sender};

use crate::csgofloat::CsgoFloatClient;
use crate::steam::{MarketPriceClient, UnhydratedUnlock, Unlock};
use crate::store::Store;

pub struct Handler {
    events_rx: Receiver<Option<UnhydratedUnlock>>,
    events_tx: Arc<Sender<Option<UnhydratedUnlock>>>,

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

pub async fn handle_state(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

pub async fn handle_websocket(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}

pub async fn handle_upload(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder().body(Body::empty()).unwrap())
}
