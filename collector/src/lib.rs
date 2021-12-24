use std::convert::Infallible;
use std::sync::Arc;

use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};
use reqwest::cookie::Jar;
use reqwest::{Client, StatusCode, Url};
use scraper::{ElementRef, Html, Selector};

static COLLECTION_URL: &str = "https://";

pub struct Collector {
    user_friendly_name: String,
    auth_cookie: String,

    client: SteamClient,

    last_unboxing: Option<DateTime<Utc>>,
    last_unboxed: Option<InventoryId>,
}

impl Collector {}

#[derive(PartialEq)]
pub struct InventoryId {
    pub class_id: u64,
    pub instance_id: u64,
}

pub struct Unlock {
    pub required_key: bool,
    pub case_name: String,
    pub item: Item,
}

pub struct UnhydratedUnlock {
    pub case: InventoryId,
    pub key: Option<InventoryId>,
    pub item: InventoryId,

    pub at: DateTime<
}

pub struct UnhydratedItem {
    pub id: InventoryId,
}

pub struct Item {
    pub name: String,
    pub float: f64,
    pub id: InventoryId,
    pub variant: String,
}

struct SteamCredentials {
    session_id: String,
    login_token: String,
}

impl SteamCredentials {
    pub fn into_jar(self) -> Jar {
        let jar = Jar::default();
        let url = "https://steamcommunity.com".parse().unwrap();
        let cookie_str = format!(
            "sessionid={}; steamLoginSecure={}",
            self.session_id, self.login_token
        );
        jar.add_cookie_str(&cookie_str, &url);

        jar
    }
}

struct SteamClient {
    username: String,
    user_id: u64,
    http_client: Client,

    inventory_url: Url,
    inventory_history_url: Url,
}

pub enum FetchNewItemsError {
    TransportError(reqwest::Error),
    AuthenticationFailure,
    UnhandledStatusCode(StatusCode),
    NoHistoryFound,
}

impl From<reqwest::Error> for FetchNewItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::TransportError(e)
    }
}

impl SteamClient {
    pub fn new(
        username: String,
        user_id: u64,
        creds: SteamCredentials,
    ) -> Result<Self, Infallible> {
        let http_client = Client::builder()
            .cookie_provider(Arc::new(creds.into_jar()))
            .build()
            .unwrap();

        let inventory_url = format!(
            "https://steamcommunity.com/inventory/{}/730/2?l=english&count=25",
            user_id
        )
        .parse()
        .unwrap();
        let inventory_history_url = format!(
            "https://steamcommunity.com/id/{}/inventoryhistory/?app[]=730",
            username
        )
        .parse()
        .unwrap();

        Ok(Self {
            username,
            user_id,
            http_client,

            inventory_url,
            inventory_history_url,
        })
    }

    pub async fn fetch_new_items(
        &mut self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<UnhydratedUnlock>, FetchNewItemsError> {
        let resp = self
            .http_client
            .get(self.inventory_history_url.clone())
            .send()
            .await?;
        let status = resp.status();

        match status {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                return Err(FetchNewItemsError::AuthenticationFailure)
            }
            _ => return Err(FetchNewItemsError::UnhandledStatusCode(status)),
        }

        let data = resp.text().await?;
        let parsed_data = Html::parse_document(&data);

        // TODO: lazy_static?
        let trade_selector = Selector::parse("div.tradehistoryrow").unwrap();
        let trade_date_selector = Selector::parse("div.tradehistory_date").unwrap();
        let description_selector = Selector::parse("div.tradehistory_event_description").unwrap();
        let info_selector = Selector::parse("div.tradehistory_items").unwrap();
        let trade_item_selector = Selector::parse("span.history_item").unwrap();
        let trades = parsed_data.select(&trade_selector);
        let mut seen_any = false;

        let mut unlocks: Vec<UnhydratedUnlock> = Vec::new();

        for trade in trades {
            let desc = match trade.select(&description_selector).next() {
                Some(d) => d,
                // TODO: Should be a parse error
                None => continue,
            };

            seen_any = true;

            let desc_text = match desc.text().next() {
                Some(t) => t,
                // TODO: Should be a parse error
                None => continue,
            }
            .trim();

            if desc_text != "Unlocked a container" {
                // This transaction was not a container unboxing
                continue;
            }

            let mut date_nodes = match trade.select(&trade_date_selector).next() {
                Some(d) => d.text(),
                // TODO: Should be a parse error
                None => continue,
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
                return Ok(unlocks);
            }

            let mut sides = trade.select(&info_selector);
            let mut lost_items = sides.next().unwrap().select(&trade_item_selector);
            let case_node = lost_items.next().unwrap();
            let key_node = lost_items.next();

            let gained_item = sides
                .next()
                .unwrap()
                .select(&trade_item_selector)
                .next()
                .unwrap();

            unlocks.push(UnhydratedUnlock{
                case: inv_id_from_node(case_node),
                key: key_node.map(inv_id_from_node),
                item: inv_id_from_node(gained_item),

                at: datetime,
            });
        }

        if !seen_any {
            return Err(FetchNewItemsError::NoHistoryFound);
        }

        unimplemented!()
    }
}

fn inv_id_from_node<'a>(r: ElementRef<'a>) -> InventoryId {
    let v = r.value();
    let class_id = v.attr("data-classid").unwrap().parse().unwrap();
    let instance_id = v.attr("data-instanceid").unwrap().parse().unwrap();

    InventoryId { class_id, instance_id}
}
