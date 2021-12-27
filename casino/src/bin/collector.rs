use std::env;

use chrono::{NaiveDate, TimeZone, Utc};

use casino::collector::Collector;
use casino::steam::SteamCredentials;

#[tokio::main]
async fn main() {
    let username = env::var("STEAM_USERNAME").expect("STEAM_USERNAME unset");
    let session_id = env::var("STEAM_SESSION_ID")
        .expect("STEAM_SESSION_ID unset")
        .parse()
        .unwrap();
    let login_token = env::var("STEAM_TOKEN").expect("STEAM_TOKEN unset");

    let creds = SteamCredentials::new(session_id, login_token);

    let naive_start_time = NaiveDate::from_ymd(2021, 11, 21).and_hms(0, 0, 0);
    let start_time = Utc.from_local_datetime(&naive_start_time).unwrap();

    Collector::new(username, creds, Some(start_time))
        .await
        .unwrap()
        .run()
        .await
        .unwrap();
}
