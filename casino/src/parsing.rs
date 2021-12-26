use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use scraper::{ElementRef, Selector};
use serde::{Deserialize, Serialize};

lazy_static::lazy_static! {
    pub static ref TRADE_SELECTOR: Selector = Selector::parse("div.tradehistoryrow").unwrap();
    pub static ref TRADE_DATE_SELECTOR: Selector = Selector::parse("div.tradehistory_date").unwrap();
    pub static ref DESCRIPTION_SELECTOR: Selector = Selector::parse("div.tradehistory_event_description").unwrap();
    pub static ref INFO_SELECTOR: Selector = Selector::parse("div.tradehistory_items").unwrap();
    pub static ref TRADE_ITEM_SELECTOR: Selector = Selector::parse(".history_item").unwrap();
    pub static ref TRADE_ITEM_IMG_SELECTOR: Selector = Selector::parse("img.tradehistory_received_item_img").unwrap();
    pub static ref TRADE_ITEM_NAME_SELECTOR: Selector = Selector::parse("span.history_item_name").unwrap();
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub id: InventoryId,
    pub variant: String,
    pub icon_url: String,
}

pub struct RawUnlock {
    pub case: TrivialItem,
    pub key: Option<TrivialItem>,
    pub item: InventoryId,

    pub at: DateTime<Utc>,
}

pub enum ParseResult {
    Success(RawUnlock),
    TooOld,
    WrongTransactionType,
    Unparseable,
}

pub fn parse_raw_unlock(trade: ElementRef<'_>, since: Option<&DateTime<Utc>>) -> ParseResult {
    let desc = match trade.select(&DESCRIPTION_SELECTOR).next() {
        Some(d) => d,
        // TODO: Should convey more info
        None => return ParseResult::Unparseable,
    };

    let desc_text = match desc.text().next() {
        Some(t) => t,
        // TODO: Should convey more info
        None => return ParseResult::Unparseable,
    }
    .trim();

    if desc_text != "Unlocked a container" {
        // This transaction was not a container unboxing
        return ParseResult::WrongTransactionType;
    }

    let mut date_nodes = match trade.select(&TRADE_DATE_SELECTOR).next() {
        Some(d) => d.text(),
        // TODO: Should convey more info
        None => return ParseResult::Unparseable,
    };

    // TODO: Parser errors
    let date = date_nodes.next().unwrap().trim();
    let time = date_nodes.next().unwrap().trim();
    let datetime = NaiveDateTime::parse_from_str(
        // Oct 31, 2021 1:50pm
        format!("{} {}", date, time).as_ref(),
        "%b %e, %Y %l:%M%P",
    )
    .unwrap();
    let datetime = Utc.from_local_datetime(&datetime).unwrap();

    if since.map(|s| &datetime < s).unwrap_or(false) {
        // We have successfully started parsing a trade that is older than our threshold, return
        // early.
        return ParseResult::TooOld;
    }

    let mut sides = trade.select(&INFO_SELECTOR);
    let mut lost_items = sides.next().unwrap().select(&TRADE_ITEM_SELECTOR);
    let case_node = lost_items.next().unwrap();
    let key_node = lost_items.next();

    let gained_items = sides.next().unwrap();
    let gained_item = gained_items.select(&TRADE_ITEM_SELECTOR).next().unwrap();

    ParseResult::Success(RawUnlock {
        case: item_from_node(case_node, "Case".into()),
        key: key_node.map(|n| item_from_node(n, "Key".into())),
        item: inv_id_from_node(gained_item),

        at: datetime,
    })
}

fn item_from_node(r: ElementRef<'_>, variant: String) -> TrivialItem {
    let name = r
        .select(&TRADE_ITEM_NAME_SELECTOR)
        .next()
        .unwrap()
        .text()
        .next()
        .unwrap()
        .trim()
        .to_string();

    let image_id = r
        .select(&TRADE_ITEM_IMG_SELECTOR)
        .next()
        .unwrap()
        .value()
        .attr("src")
        .unwrap()
        .split('/')
        .nth(5)
        .unwrap();

    let image_url = format!(
        "https://community.cloudflare.steamstatic.com/economy/image/{}",
        image_id
    );

    TrivialItem {
        name,
        color: None,
        image_url,
    }
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
