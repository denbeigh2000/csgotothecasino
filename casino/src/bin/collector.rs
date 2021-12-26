use casino::steam::{UnhydratedUnlock, TrivialItem};
use chrono::Utc;

static CLUTCH_CASE_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFY5naqQIz4R7Yjix9bZkvKiZrmAzzlTu5AoibiT8d_x21Wy8hY_MWz1doSLMlhpM3FKbNs";
static CLUTCH_CASE_KEY_IMG: &str = "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiev8ZQQ30KubIWVDudrgkNncw6-hY-2Fkz1S7JRz2erHodnzig2xqUVvYDrtZNjCAC7WDrU";

fn main() {
    let key = TrivialItem::new("Clutch Case Key", CLUTCH_CASE_KEY_IMG, None);
    let case = TrivialItem::new("Clutch Case", CLUTCH_CASE_IMG, None);

    let item = UnhydratedUnlock {
        key: Some(key),
        case,
        item_market_link:  "steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198035933253A24028753890D2306591149808544275".into(),
        item_market_name: "Souvenir P90 | Facility Negative (Minimal Wear)".into(),
        at: Utc::now(),
        name: "denbeigh".into(),
    };

    let data = serde_json::to_string(&item).unwrap();
    println!("{}", data);
}
