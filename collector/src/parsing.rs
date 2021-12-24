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

pub struct UnhydratedUnlock {
    pub case: Item,
    pub key: Option<Item>,
    pub item: InventoryId,

    pub at: DateTime<Utc>,
}

pub struct UnhydratedItem {
    pub id: InventoryId,
}

pub enum ParseResult {
    Success(UnhydratedUnlock),
    TooOld,
    WrongTransactionType,
    Unparseable,
}

pub fn parse_unhydrated_unlock(trade: ElementRef<'_>, since: &DateTime<Utc>) -> ParseResult {
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

    if &datetime < since {
        // We have successfully started parsing a trade that is older than our threshold, return
        // early.
        return ParseResult::TooOld;
    }

    let mut sides = trade.select(&INFO_SELECTOR);
    let mut lost_items = sides.next().unwrap().select(&TRADE_ITEM_SELECTOR);
    let case_node = lost_items.next().unwrap();
    let key_node = lost_items.next();

    dbg!("{:?}", &*case_node.value());
    dbg!("{:?}", key_node.map(|n| &*n.value()));

    let gained_items = sides.next().unwrap();

    dbg!("{:?}", &gained_items.value());

    let gained_item = gained_items.select(&TRADE_ITEM_SELECTOR).next().unwrap();

    ParseResult::Success(UnhydratedUnlock {
        case: item_from_node(case_node, "Case".into()),
        key: key_node.map(|n| item_from_node(n, "Key".into())),
        item: inv_id_from_node(gained_item),

        at: datetime,
    })
}

fn item_from_node(r: ElementRef<'_>, variant: String) -> Item {
    let name = r
        .select(&TRADE_ITEM_NAME_SELECTOR)
        .next()
        .unwrap()
        .text()
        .next()
        .unwrap()
        .trim()
        .to_string();

    let icon_url = r
        .select(&TRADE_ITEM_IMG_SELECTOR)
        .next()
        .unwrap()
        .value()
        .attr("src")
        .unwrap()
        .split('/')
        .nth(5)
        .unwrap()
        .to_string();

    let id = inv_id_from_node(r);

    Item {
        name,
        id,
        variant,
        icon_url,
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
