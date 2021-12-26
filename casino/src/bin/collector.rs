use std::env;

use casino::collector::Collector;
use casino::steam::SteamCredentials;

#[tokio::main]
async fn main() {
    let username = env::var("STEAM_USERNAME").expect("STEAM_USERNAME unset");
    let steam_id = env::var("STEAM_ID")
        .expect("STEAM_ID unset")
        .parse()
        .unwrap();
    let session_id = env::var("STEAM_SESSION_ID")
        .expect("STEAM_SESSION_ID unset")
        .parse()
        .unwrap();
    let login_token = env::var("STEAM_TOKEN").expect("STEAM_TOKEN unset");

    let creds = SteamCredentials::new(session_id, login_token);

    Collector::new(username, steam_id, creds, None)
        .unwrap()
        .run()
        .await
        .unwrap();
}
