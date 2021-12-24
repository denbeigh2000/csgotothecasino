use chrono::{DateTime, Utc};

use crate::parsing::InventoryId;
use crate::steam::SteamClient;

pub mod aggregator;
mod parsing;
mod csgofloat;
mod steam;
mod store;

static COLLECTION_URL: &str = "https://";

pub struct Collector {
    user_friendly_name: String,
    auth_cookie: String,

    client: SteamClient,

    last_unboxing: Option<DateTime<Utc>>,
    last_unboxed: Option<InventoryId>,
}

impl Collector {}
