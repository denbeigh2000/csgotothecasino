use std::fmt::{self, Display};
use std::num::ParseIntError;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use scraper::element_ref::Text;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct InventoryId {
    pub class_id: u64,
    pub instance_id: u64,
}

impl InventoryId {
    pub fn new(class_id: u64, instance_id: u64) -> Self {
        Self {
            class_id,
            instance_id,
        }
    }
}

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

#[derive(Debug)]
pub enum ParseFailure {
    MissingDescription,
    MissingDescriptionText,
    MissingDate,
    MissingTime,
    DateFormattingChanged,
    MissingLostItems,
    MissingLostCase,
    MissingGainedItems,
    MissingGainedItem,
    MissingTradeId,
    TradeIdFormattingChanged,
    TrivialItemParseError(TrivialItemParseError),
}

impl Display for ParseFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error parsing inventory history: ")?;
        match self {
            Self::MissingDescription => write!(f, "could not find trade description"),
            Self::MissingDescriptionText => write!(f, "could not find trade description text"),
            Self::MissingDate => write!(f, "could not find trade date"),
            Self::MissingTime => write!(f, "could not find trade time"),
            Self::DateFormattingChanged => write!(f, "could not parse date from existing format"),
            Self::MissingLostItems => write!(f, "could not find lost items from unboxing"),
            Self::MissingLostCase => write!(f, "could not used container from unboxing"),
            Self::MissingGainedItems => write!(f, "could not find items gained from unboxing"),
            Self::MissingGainedItem => write!(f, "could not find item gained from unboxing"),
            Self::MissingTradeId => write!(f, "could not find id associated with trade"),
            Self::TradeIdFormattingChanged => write!(f, "could not parse trade id from element"),
            Self::TrivialItemParseError(e) => write!(f, "could not parse trivial item: {}", e),
        }
    }
}

impl From<TrivialItemParseError> for ParseFailure {
    fn from(e: TrivialItemParseError) -> Self {
        Self::TrivialItemParseError(e)
    }
}

#[derive(Debug)]
pub enum TrivialItemParseError {
    MissingNameNode,
    MissingNameText,
    MissingImageNode,
    MissingImageText,
    ImageURLFormatChanged,
}

impl Display for TrivialItemParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error parsing trivial item: ")?;
        match self {
            Self::MissingNameNode => write!(f, "could not find item name node"),
            Self::MissingNameText => write!(f, "could not find item name text"),
            Self::MissingImageNode => write!(f, "could not find item image node"),
            Self::MissingImageText => write!(f, "could not find item image text"),
            Self::ImageURLFormatChanged => write!(f, "image url format has changed"),
        }
    }
}

#[derive(Debug)]
pub enum UserIdParseError {
    MissingUserIdElement,
    BadUserId(ParseIntError),
    UserIdElementParseError,
}

impl From<ParseIntError> for UserIdParseError {
    fn from(e: ParseIntError) -> Self {
        Self::BadUserId(e)
    }
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

#[derive(Debug)]
pub enum AuthenticationParseError {
    MissingSteamLoginArea,
    MissingLoginOrUserInfo,
}

impl Display for AuthenticationParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSteamLoginArea => write!(f, "could not find login area on page"),
            Self::MissingLoginOrUserInfo => write!(f, "found login area, but not indicator"),
        }
    }
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

pub fn parse_raw_unlock(
    trade: ElementRef<'_>,
    since: Option<&DateTime<Utc>>,
    last_seen_id: Option<&str>,
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
    let datetime = Utc.from_local_datetime(&datetime).unwrap();

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

    let history_id_attr = case_node.value().id().unwrap();
    let history_id = HISTORY_ID_REGEX
        .captures(history_id_attr)
        .ok_or(ParseFailure::MissingTradeId)?
        .get(1)
        .ok_or(ParseFailure::TradeIdFormattingChanged)?
        .as_str();

    if last_seen_id.map(|l| l == history_id).unwrap_or(false) {
        return Ok(ParseSuccess::TooOld);
    }
    let key = key_node.map(item_from_node).transpose()?;

    Ok(ParseSuccess::ValidItem(RawUnlock {
        history_id: history_id.to_string(),

        case: item_from_node(case_node)?,
        key,
        item: inv_id_from_node(gained_item),

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
