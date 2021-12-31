use std::collections::HashMap;
use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use reqwest::{Client, Request, StatusCode};
use reqwest::header::COOKIE;
use scraper::Html;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::errors::{
    AuthenticationCheckError,
    FetchItemsError,
    FetchNewUnpreparedItemsError,
    PrepareItemsError,
};
pub use crate::id::{Id, IdUrlParseError};
use crate::parsing::{
    is_authenticated,
    parse_raw_unlock,
    InventoryId,
    ParseSuccess,
    RawUnlock,
    TrivialItem,
    TRADE_SELECTOR,
};

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

#[derive(Debug)]
pub enum CredentialParseError {
    NoSessionId,
    DoesNotResembleCookie,
}

impl Display for CredentialParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialParseError::NoSessionId => {
                writeln!(f, "could not parse session id.")?;
                write!(f, "ensure you are passing a `sessionid` parameter")
            }
            CredentialParseError::DoesNotResembleCookie => {
                write!(f, "given string does not resemble a cookie")
            }
        }
    }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryDescription {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "classid"))]
    pub class_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "instanceid"))]
    pub instance_id: u64,
    pub icon_url: String,
    #[serde(rename(deserialize = "market_hash_name"))]
    pub name: String,
    #[serde(rename = "type")]
    pub variant: String,

    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Asset {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "appid"))]
    app_id: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "assetid"))]
    asset_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "classid"))]
    class_id: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "instanceid"))]
    instance_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Action {
    link: String,
    name: String,
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

    fn inv_history_req(&self) -> Request {
        self.http_client
            .get(self.id.inventory_history_url())
            .header("Cookie", &self.cookie_str)
            .build()
            .unwrap()
    }

    pub async fn fetch_new_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_id: Option<&str>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchItemsError> {
        let unhydrated = self.fetch_new_unprepared_items(since, last_id).await?;
        let prepared = self
            .prepare_unlocks(unhydrated, self.username.clone())
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

    async fn fetch_new_unprepared_items(
        &self,
        since: Option<&DateTime<Utc>>,
        last_id: Option<&str>,
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
            match parse_raw_unlock(trade, since, last_id)? {
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

    async fn prepare_unlocks(
        &self,
        items: Vec<RawUnlock>,
        name: String,
    ) -> Result<Vec<UnhydratedUnlock>, PrepareItemsError> {
        let resp = self
            .http_client
            .get(self.id.inventory_url())
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let inv: Inventory = serde_json::from_str(&resp)?;
        let data_map: HashMap<InventoryId, InventoryDescription> = inv
            .descriptions
            .into_iter()
            .fold(HashMap::new(), |mut acc, item| {
                let id = InventoryId::new(item.class_id, item.instance_id);
                acc.insert(id, item);
                acc
            });

        let asset_map: HashMap<InventoryId, Asset> =
            inv.assets
                .into_iter()
                .fold(HashMap::new(), |mut acc, item| {
                    let id = InventoryId::new(item.class_id, item.instance_id);
                    acc.insert(id, item);
                    acc
                });

        let results = items
            .into_iter()
            .map(|i| {
                let case = i.case;
                let key = i.key;
                let item_data = data_map.get(&i.item).unwrap();
                let item_asset = asset_map.get(&i.item).unwrap();

                let item_market_name = item_data.name.clone();
                let actions = item_data.actions.as_ref().unwrap();
                let link_tpl = actions
                    .iter()
                    .find(|a| {
                        a.name.starts_with("Inspect") && a.link.starts_with("steam://rungame/730/")
                    })
                    .unwrap();

                let item_market_link = link_tpl
                    .link
                    .replacen("%assetid%", &item_asset.asset_id.to_string(), 1)
                    .replacen("%owner_steamid%", &self.id.user_id().to_string(), 1);

                let history_id = i.history_id;
                let at = i.at;
                let name = name.clone();

                UnhydratedUnlock {
                    history_id,

                    key,
                    case,
                    item_market_link,
                    item_market_name,
                    at,
                    name,
                }
            })
            .collect();

        Ok(results)
    }
}
