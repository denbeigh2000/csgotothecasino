use std::collections::HashMap;
use std::env;

use bb8_redis::redis::ConnectionInfo;

use casino::aggregator::keystore::KeyStore;
use casino::aggregator::{serve, Handler};
use casino::csgofloat::CsgoFloatClient;
use casino::steam::MarketPriceClient;
use casino::store::Store;

#[tokio::main]
async fn main() {
    #[cfg(not(feature = "not-stub"))]
    let make_handler = || async { Handler::default() };

    #[cfg(feature = "not-stub")]
    let make_handler = || async {
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL unset");
        let csgofloat_key = env::var("CSGOFLOAT_KEY").expect("CSGOFLOAT_KEY unset");
        let info: ConnectionInfo = redis_url
            .parse()
            .unwrap_or_else(|_| panic!("not a valid redis url: {}", redis_url));

        let store = Store::new(info.clone()).await.unwrap();
        let mut dev_keys: HashMap<String, String> = HashMap::new();
        dev_keys.insert("denbeigh2000".to_string(), "denbeigh".to_string());
        dev_keys.insert("badcop_".to_string(), "denbeigh".to_string());
        let keystore = KeyStore::new(dev_keys);
        let csgo_float = CsgoFloatClient::new(csgofloat_key, info.clone())
            .await
            .unwrap();
        let market_price_client = MarketPriceClient::new(info).await.unwrap();

        Handler::new(store, keystore, csgo_float, market_price_client)
    };

    serve(make_handler).await.unwrap();
}
