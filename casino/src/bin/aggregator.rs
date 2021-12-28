use casino::aggregator::{serve, Handler};

#[tokio::main]
async fn main() {
    real_main().await;
}

#[cfg(not(feature = "not-stub"))]
async fn real_main() {
    serve(Handler::default()).await.unwrap();
}

#[cfg(feature = "not-stub")]
async fn real_main() {
    use std::env;

    use bb8_redis::redis::ConnectionInfo;

    use casino::aggregator::keystore::KeyStore;
    use casino::csgofloat::CsgoFloatClient;
    use casino::steam::MarketPriceClient;
    use casino::store::Store;

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL unset");
    let csgofloat_key = env::var("CSGOFLOAT_KEY").expect("CSGOFLOAT_KEY unset");
    let info: ConnectionInfo = redis_url
        .parse()
        .unwrap_or_else(|_| panic!("not a valid redis url: {}", redis_url));

    let store = Store::new(info.clone()).await.unwrap();
    let keystore = KeyStore::load_from_file("./keystore.yaml").await.unwrap();
    let csgo_float = CsgoFloatClient::new(csgofloat_key, info.clone())
        .await
        .unwrap();
    let market_price_client = MarketPriceClient::new(info).await.unwrap();

    let h = Handler::new(store, keystore, csgo_float, market_price_client);

    serve(h).await.unwrap();
}
