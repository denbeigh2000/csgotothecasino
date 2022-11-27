use std::num::ParseIntError;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use thiserror::Error;

lazy_static::lazy_static! {
    pub static ref LOGIN_AREA_SELECTOR: Selector = Selector::parse("#global_actions").unwrap();
    pub static ref LOGGED_IN_ACTION_SELECTOR: Selector = Selector::parse("#account_pulldown").unwrap();
    pub static ref LOGGED_OUT_ACTION_SELECTOR: Selector = Selector::parse("#language_pulldown").unwrap();

    pub static ref USER_ID_SELECTOR: Selector = Selector::parse("div.commentthread_area").unwrap();

    pub static ref TRADE_SELECTOR: Selector = Selector::parse("div.tradehistoryrow").unwrap();
    pub static ref TRADE_DATE_SELECTOR: Selector = Selector::parse("div.tradehistory_date").unwrap();
    pub static ref DESCRIPTION_SELECTOR: Selector = Selector::parse("div.tradehistory_event_description").unwrap();
    pub static ref INFO_SELECTOR: Selector = Selector::parse("div.tradehistory_items").unwrap();
    pub static ref TRADE_ITEM_SELECTOR: Selector = Selector::parse(".history_item").unwrap();
    pub static ref TRADE_ITEM_IMG_SELECTOR: Selector = Selector::parse("img.tradehistory_received_item_img").unwrap();
    pub static ref TRADE_ITEM_NAME_SELECTOR: Selector = Selector::parse("span.history_item_name").unwrap();

    pub static ref HISTORY_ID_REGEX: Regex = Regex::new(r"^history([0-9a-f]{40})_.+").unwrap();
    pub static ref USER_ID_REGEX: Regex = Regex::new("commentthread_Profile_([0-9]+)_.*").unwrap();
}

/// Represents some non-unique item on the Steam Market (keys, cases, etc)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrivialItem {
    name: String,
    color: Option<String>,
    image_url: String,
}

impl TrivialItem {
    pub fn new<S1, S2>(name: S1, image_url: S2, color: Option<String>) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        let name = name.into();
        let image_url = image_url.into();
        Self {
            name,
            image_url,
            color,
        }
    }
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
}

/// Minimal representation of a unique item in a user's inventory
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct InventoryId {
    pub class_id: u64,
    pub instance_id: u64,
}

impl From<&InventoryDescription> for InventoryId {
    fn from(item: &InventoryDescription) -> Self {
        Self {
            class_id: item.class_id,
            instance_id: item.instance_id,
        }
    }
}

impl From<&Asset> for InventoryId {
    fn from(item: &Asset) -> Self {
        Self {
            class_id: item.class_id,
            instance_id: item.instance_id,
        }
    }
}

impl InventoryId {
    pub fn new(class_id: u64, instance_id: u64) -> Self {
        Self {
            class_id,
            instance_id,
        }
    }
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
pub struct Action {
    pub link: String,
    pub name: String,
}

impl Action {
    pub fn is_csgo_inspect_link(&self) -> bool {
        self.name.starts_with("Inspect") && self.link.starts_with("steam://rungame/730/")
    }
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

impl Asset {
    pub fn asset_id(&self) -> &u64 {
        &self.asset_id
    }
}

/// Represents a transaction fetched from the Inventory History page.
#[derive(Debug)]
pub struct RawUnlock {
    pub history_id: String,

    pub case: TrivialItem,
    pub key: Option<TrivialItem>,
    pub item: InventoryId,

    pub at: DateTime<Utc>,
}

pub type ParseResult = Result<ParseSuccess, ParseFailure>;

pub enum ParseSuccess {
    ValidItem(RawUnlock),
    TooOld,
    WrongTransactionType,
}

#[derive(Debug, Error)]
pub enum ParseFailure {
    #[error("could not find trade description")]
    MissingDescription,
    #[error("could not find trade description text")]
    MissingDescriptionText,
    #[error("could not find trade date")]
    MissingDate,
    #[error("could not find trade time")]
    MissingTime,
    #[error("could not parse date from existing format")]
    DateFormattingChanged,
    #[error("could not find lost items from unboxing")]
    MissingLostItems,
    #[error("could not used container from unboxing")]
    MissingLostCase,
    #[error("could not find items gained from unboxing")]
    MissingGainedItems,
    #[error("could not find item gained from unboxing")]
    MissingGainedItem,
    #[error("could not find id associated with trade")]
    MissingTradeId,
    #[error("could not parse trade id from element")]
    TradeIdFormattingChanged,
    #[error("could not parse trivial item: {0}")]
    TrivialItemParsing(#[from] TrivialItemParseError),
}

#[derive(Debug, Error)]
pub enum TrivialItemParseError {
    #[error("could not find item name node")]
    MissingNameNode,
    #[error("could not find item name text")]
    MissingNameText,
    #[error("could not find item image node")]
    MissingImageNode,
    #[error("could not find item image text")]
    MissingImageText,
    #[error("image url format has changed")]
    ImageURLFormatChanged,
}

#[derive(Debug, Error)]
pub enum UserIdParseError {
    #[error("could not find user id element")]
    MissingUserIdElement,
    #[error("error parsing user id: {0}")]
    BadUserId(#[from] ParseIntError),
    #[error("error parsing steam user id")]
    UserIdElementParseError,
}

pub fn get_userid(page: &Html) -> Result<u64, UserIdParseError> {
    let user_id_element = page
        .select(&USER_ID_SELECTOR)
        .next()
        .ok_or(UserIdParseError::MissingUserIdElement)?;

    let element_id = user_id_element
        .value()
        .id()
        .ok_or(UserIdParseError::MissingUserIdElement)?;

    match USER_ID_REGEX.captures(element_id) {
        Some(g) => Ok(g.get(1).unwrap().as_str().parse()?),
        None => Err(UserIdParseError::UserIdElementParseError),
    }
}

#[derive(Debug, Error)]
pub enum AuthenticationParseError {
    #[error("could not find login area on page")]
    MissingSteamLoginArea,
    #[error("found login area, but not indicator")]
    MissingLoginOrUserInfo,
}

pub fn is_authenticated(page: &Html) -> Result<bool, AuthenticationParseError> {
    let login_area = page
        .select(&LOGIN_AREA_SELECTOR)
        .next()
        .ok_or(AuthenticationParseError::MissingSteamLoginArea)?;

    if login_area
        .select(&LOGGED_IN_ACTION_SELECTOR)
        .next()
        .is_some()
    {
        return Ok(true);
    }
    if login_area
        .select(&LOGGED_OUT_ACTION_SELECTOR)
        .next()
        .is_some()
    {
        return Ok(false);
    }

    Err(AuthenticationParseError::MissingLoginOrUserInfo)
}

// TODO: This probably shouldn't concern itself with parsing _and_ filtering.
pub fn parse_raw_unlock(
    trade: ElementRef<'_>,
    since: Option<&DateTime<Utc>>,
    last_seen_inventory_id: Option<&InventoryId>,
) -> ParseResult {
    let desc = trade
        .select(&DESCRIPTION_SELECTOR)
        .next()
        .ok_or(ParseFailure::MissingDescription)?;
    let desc_text = desc
        .text()
        .next()
        .ok_or(ParseFailure::MissingDescriptionText)?
        .trim();

    // NOTE: We could easily(?) change this to handle trade-ups, too.
    if desc_text != "Unlocked a container" {
        // This transaction was not a container unboxing
        return Ok(ParseSuccess::WrongTransactionType);
    }

    let mut date_nodes = trade
        .select(&TRADE_DATE_SELECTOR)
        .next()
        .ok_or(ParseFailure::MissingDate)?
        .text();

    let date = date_nodes
        .next()
        .map(|i| i.trim())
        .ok_or(ParseFailure::MissingDate)?;
    let time = date_nodes
        .next()
        .map(|i| i.trim())
        .ok_or(ParseFailure::MissingTime)?;
    let datetime = NaiveDateTime::parse_from_str(
        // Oct 31, 2021 1:50pm
        format!("{} {}", date, time).as_ref(),
        "%b %e, %Y %l:%M%P",
    )
    .map_err(|_| ParseFailure::DateFormattingChanged)?;
    let datetime = Utc
        .from_local_datetime(&datetime)
        .unwrap();

    if since.map(|s| &datetime < s).unwrap_or(false) {
        // We have successfully started parsing a trade that is older than our threshold, return
        // early.
        return Ok(ParseSuccess::TooOld);
    }

    let mut sides = trade.select(&INFO_SELECTOR);
    let mut lost_items = sides
        .next()
        .ok_or(ParseFailure::MissingLostItems)?
        .select(&TRADE_ITEM_SELECTOR);
    let case_node = lost_items.next().ok_or(ParseFailure::MissingLostCase)?;
    let key_node = lost_items.next();

    let gained_items = sides.next().ok_or(ParseFailure::MissingGainedItems)?;
    let gained_item = gained_items
        .select(&TRADE_ITEM_SELECTOR)
        .next()
        .ok_or(ParseFailure::MissingGainedItem)?;

    let inv_id = inv_id_from_node(gained_item);

    let history_id_attr = case_node.value().id().unwrap();
    let history_id = HISTORY_ID_REGEX
        .captures(history_id_attr)
        .ok_or(ParseFailure::MissingTradeId)?
        .get(1)
        .ok_or(ParseFailure::TradeIdFormattingChanged)?
        .as_str();

    // TODO: Want to check to see if asset_id is monotonically increasing
    if last_seen_inventory_id.map(|l| l == &inv_id).unwrap_or(false) {
        return Ok(ParseSuccess::TooOld);
    }
    let key = key_node.map(item_from_node).transpose()?;

    Ok(ParseSuccess::ValidItem(RawUnlock {
        history_id: history_id.to_string(),

        case: item_from_node(case_node)?,
        key,
        item: inv_id,

        at: datetime,
    }))
}

fn item_from_node(r: ElementRef<'_>) -> Result<TrivialItem, TrivialItemParseError> {
    let name = r
        .select(&TRADE_ITEM_NAME_SELECTOR)
        .next()
        .ok_or(TrivialItemParseError::MissingNameNode)?
        .text()
        .next()
        .ok_or(TrivialItemParseError::MissingNameText)?
        .trim()
        .to_string();

    let image_id = r
        .select(&TRADE_ITEM_IMG_SELECTOR)
        .next()
        .ok_or(TrivialItemParseError::MissingImageNode)?
        .value()
        .attr("src")
        .ok_or(TrivialItemParseError::MissingImageText)?
        .split('/')
        .nth(5)
        .unwrap();

    let image_url = format!(
        "https://community.cloudflare.steamstatic.com/economy/image/{}",
        image_id
    );

    Ok(TrivialItem {
        name,
        color: None,
        image_url,
    })
}

fn inv_id_from_node(r: ElementRef<'_>) -> InventoryId {
    let v = r.value();
    let class_id = v.attr("data-classid").unwrap().parse().unwrap();
    let instance_id = v.attr("data-instanceid").unwrap().parse().unwrap();

    InventoryId {
        class_id,
        instance_id,
    }
}
