use std::collections::HashMap;

use chrono::{DateTime, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use reqwest::header::COOKIE;
use reqwest::{Client, Request, StatusCode};
use scraper::Html;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::errors::{
    AuthenticationCheckError, FetchInventoryError, FetchItemsError, FetchNewUnpreparedItemsError,
    LocalPrepareError, PrepareItemsError,
};
pub use crate::id::{Id, IdUrlParseError};
use crate::parsing::{
    is_authenticated, parse_raw_unlock, Asset, ParseSuccess, RawUnlock, TrivialItem, TRADE_SELECTOR,
};
pub use crate::parsing::{InventoryDescription, InventoryId};

pub mod errors;
mod id;
#[cfg(feature = "backend")]
mod redis;
#[cfg(feature = "backend")]
pub use self::redis::*;
#[cfg(feature = "backend")]
mod price_client;
#[cfg(feature = "backend")]
pub use price_client::*;
mod parsing;

lazy_static::lazy_static! {
    static ref COOKIE_REGEX: Regex = Regex::new(r"[^\s=;]+=[^\s=;]+").unwrap();
}

type LocalPrepareResult = Result<UnhydratedUnlock, LocalPrepareError>;

/// A transaction with additional
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnhydratedUnlock {
    pub history_id: String,

    pub key: Option<TrivialItem>,
    pub case: TrivialItem,
    pub item_market_link: String,
    pub item_market_name: String,

    pub at: DateTime<Utc>,
    pub name: String,
}

#[derive(Debug, Error)]
pub enum CredentialParseError {
    #[error("could not parse session id, ensure you are passing a valid sessionid parameter")]
    NoSessionId,
    #[error("given string does not resemble a cookie")]
    DoesNotResembleCookie,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SteamCredentials {
    session_id: String,
    // NOTE: Unsure if this is required on accounts without steam guard
    login_token: Option<String>,
}

fn maybe_url_encode(s: String) -> String {
    if s.contains('%') {
        return s;
    }

    return utf8_percent_encode(s.as_str(), NON_ALPHANUMERIC).to_string();
}

impl SteamCredentials {
    pub fn new(session_id: String, login_token: String) -> Self {
        let login_token = Some(login_token);
        Self {
            session_id,
            login_token,
        }
    }

    pub fn try_from_cookie_str<S: AsRef<str>>(cookie_str: S) -> Result<Self, CredentialParseError> {
        let mut session_id: Option<String> = None;
        let mut login_token: Option<String> = None;
        let mut cookies = COOKIE_REGEX.find_iter(cookie_str.as_ref()).peekable();
        if cookies.peek().is_none() {
            return Err(CredentialParseError::DoesNotResembleCookie);
        }

        for cookie in cookies {
            // unwrap should be safe - we are guaranteed exactly one = from
            // our regex matching.
            match cookie.as_str().split_once('=').unwrap() {
                ("sessionid", v) => session_id = Some(v.to_string()),
                ("steamLoginSecure", v) => login_token = Some(maybe_url_encode(v.to_string())),
                _ => (),
            };
        }

        match session_id {
            Some(session_id) => Ok(Self {
                session_id,
                login_token,
            }),
            None => Err(CredentialParseError::NoSessionId),
        }
    }

    pub fn as_string(&self) -> String {
        match self.login_token.as_deref() {
            Some(t) => format!("sessionid={}; steamLoginSecure={}", self.session_id, t),
            None => format!("sessionid={}", self.session_id),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Inventory {
    assets: Vec<Asset>,
    descriptions: Vec<InventoryDescription>,
}

pub struct SteamClient {
    id: Id,
    http_client: Client,
    username: String,

    cookie_str: String,
}

impl SteamClient {
    pub fn new(id: Id, creds: SteamCredentials) -> Self {
        let http_client = Client::builder().build().unwrap();
        let cookie_str = creds.as_string();
        let username = String::from("");

        Self {
            id,
            http_client,
            username,
            cookie_str,
        }
    }

    fn inv_req(&self) -> Request {
        self.http_client
            .get(self.id.inventory_url())
            // NOTE: Desired to avoid exceeding rate limits (anon inventory
            // requests are aggressively rate-limited)
            .header("Cookie", &self.cookie_str)
            .build()
            .unwrap()
    }

    fn inv_history_req(&self) -> Request {
        self.http_client
            .get(self.id.inventory_history_url())
            .header("Cookie", &self.cookie_str)
            .build()
            .unwrap()
    }

    pub async fn fetch_history_for_new_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_item: Option<&InventoryId>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchItemsError> {
        // TODO: Need to check what exactly start_assetid does (but we should
        // have it handy by our stored InventoryId if needed)
        // TODO: This needs to fetch inventory first to avoid excceding rate limits.
        // Re-use inventory data when calling prepare_unlocks
        let inv = self.fetch_inventory().await?;
        match (inv.descriptions.first(), last_item) {
            // Return early if we have made a successful call and it shows we
            // have no new items in our inventory.
            (Some(new), Some(old)) => {
                let new_inv_id = InventoryId::from(new);
                if &new_inv_id == old {
                    // No new items to process
                    return Ok(vec![]);
                }
            }
            // Return early if steam tells us there are no items in our inventory.
            (None, _) => return Ok(vec![]),
            _ => (),
        };

        // TODO: Give this a better name
        let unhydrated = self.fetch_new_unprepared_items(since, last_item).await?;
        if unhydrated.is_empty() {
            return Ok(vec![]);
        }
        let prepared = self
            .prepare_unlocks(inv, unhydrated, self.username.clone())
            .await?;

        Ok(prepared)
    }

    pub async fn is_authenticated(&self) -> Result<bool, AuthenticationCheckError> {
        let data = self
            .http_client
            .get(self.id.profile_url())
            .header(COOKIE, &self.cookie_str)
            .send()
            .await?
            .text()
            .await?;

        let parsed = Html::parse_document(&data);
        let authenticated = is_authenticated(&parsed)?;

        Ok(authenticated)
    }

    // TODO: This needs to be adapted to use Inventory here instead of since/last_id
    async fn fetch_new_unprepared_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_item: Option<&InventoryId>,
    ) -> Result<Vec<RawUnlock>, FetchNewUnpreparedItemsError> {
        let resp = self.http_client.execute(self.inv_history_req()).await?;

        match resp.status() {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(FetchNewUnpreparedItemsError::Authentication)
            }
            status => return Err(FetchNewUnpreparedItemsError::UnhandledStatusCode(status)),
        }

        let data = resp.text().await?;
        let parsed_data = Html::parse_document(&data);

        if !is_authenticated(&parsed_data)? {
            return Err(FetchNewUnpreparedItemsError::NotAuthenticated);
        }

        let trades = parsed_data.select(&TRADE_SELECTOR);
        let mut seen_any = false;

        let mut unlocks: Vec<RawUnlock> = Vec::new();

        for trade in trades {
            // TODO: This now needs to use the data from Inventory to determine
            // age
            match parse_raw_unlock(trade, since, last_item)? {
                ParseSuccess::ValidItem(v) => unlocks.push(v),
                ParseSuccess::TooOld => return Ok(unlocks),
                ParseSuccess::WrongTransactionType => {
                    seen_any = true;
                    continue;
                }
            }
        }

        if !seen_any {
            return Err(FetchNewUnpreparedItemsError::NoHistoryFound);
        }

        Ok(unlocks)
    }

    // TODO: Account for failure modes of fetching Inventory (should be able to
    // adapt from hydration functions)
    async fn fetch_inventory(&self) -> Result<Inventory, FetchInventoryError> {
        // TODO: needs to handle unauthenticated requests as a failure mode
        let resp = self
            .http_client
            .execute(self.inv_req())
            .await?
            .error_for_status()?
            .text()
            .await?;

        let mut inv: Inventory = serde_json::from_str(&resp).map_err(FetchInventoryError::from)?;

        // NOTE: Steam returns these in oldest-first order, reverse them so
        // they're easier to work with.
        inv.assets.reverse();
        inv.descriptions.reverse();
        Ok(inv)
    }

    async fn prepare_unlocks(
        &self,
        inv: Inventory,
        items: Vec<RawUnlock>,
        name: String,
    ) -> Result<Vec<UnhydratedUnlock>, PrepareItemsError> {
        let data_map: HashMap<InventoryId, InventoryDescription> = inv
            .descriptions
            .into_iter()
            .map(|i| (InventoryId::from(&i), i))
            .collect();

        let asset_map: HashMap<InventoryId, Asset> = inv
            .assets
            .into_iter()
            .map(|i| (InventoryId::from(&i), i))
            .collect();

        // TODO: Write a comment explaining why this is here
        let (results, errs): (Vec<_>, Vec<_>) = items
            .into_iter()
            .map(|i| {
                let case = i.case;
                let key = i.key;
                let item_data = data_map
                    .get(&i.item)
                    .ok_or(LocalPrepareError::NoDescription)?;
                let item_asset = asset_map.get(&i.item).ok_or(LocalPrepareError::NoAsset)?;

                let item_market_name = item_data.name.clone();
                let actions = item_data
                    .actions
                    .as_ref()
                    .ok_or(LocalPrepareError::NoInspectLink)?;
                let link_tpl = actions
                    .iter()
                    .find(|a| a.is_csgo_inspect_link())
                    .ok_or(LocalPrepareError::NoInspectLink)?;

                let item_market_link = link_tpl
                    .link
                    .replacen("%assetid%", &item_asset.asset_id().to_string(), 1)
                    .replacen("%owner_steamid%", &self.id.user_id().to_string(), 1);

                let history_id = i.history_id;
                let at = i.at;
                let name = name.clone();

                Ok(UnhydratedUnlock {
                    history_id,

                    key,
                    case,
                    item_market_link,
                    item_market_name,
                    at,
                    name,
                })
            })
            .partition(|r: &LocalPrepareResult| r.is_ok());

        let results = results.into_iter().map(|r| r.unwrap()).collect();

        for err in errs {
            log::error!("not able to send item: {}", err.unwrap_err());
        }

        Ok(results)
    }
}
