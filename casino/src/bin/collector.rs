use std::env;

use casino::collector::Collector;
use casino::steam::SteamCredentials;

static CLUTCH_CASE_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFY5naqQIz4R7Yjix9bZkvKiZrmAzzlTu5AoibiT8d_x21Wy8hY_MWz1doSLMlhpM3FKbNs";
static CLUTCH_CASE_KEY_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiev8ZQQ30KubIWVDudrgkNncw6-hY-2Fkz1S7JRz2erHodnzig2xqUVvYDrtZNjCAC7WDrU";

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

    let mut collector = Collector::new(username, steam_id, creds, None).unwrap();
    collector.run().await.unwrap();
}
