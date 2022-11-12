use std::net::{AddrParseError, SocketAddr};

use csgofloat::{CsgoFloatClient, CsgoFloatClientCreateError};
use steam::{MarketPriceClient, MarketPriceClientCreateError};
use store::{StoreError as StoreError, Store};
use clap::{App, Arg};
use redis::{ConnectionInfo, RedisError};
use thiserror::Error;

use aggregator::keystore::{KeyStore, KeyStoreLoadSaveError};
use aggregator::{serve, Handler};

#[tokio::main]
async fn main() {
    if let Err(e) = real_main().await {
        log::error!("fatal error: {}", e);
        std::process::exit(1);
    }
}

#[derive(Debug, Error)]
enum AggregatorError {
    #[error("invalid bind address given: {0}")]
    InvalidBindIP(#[from] AddrParseError),
    #[error("invalid redis url given: {0}")]
    InvalidRedisUrl(RedisError),
    #[error("error creating csgofloat client: {0}")]
    CreatingCsgoFloatClient(#[from] CsgoFloatClientCreateError),
    #[error("error creating backing store: {0}")]
    CreatingStore(#[from] StoreError),
    #[error("error loading keystore: {0}")]
    LoadingKeystore(#[from] KeyStoreLoadSaveError),
    #[error("error creating steam market price client: {0}")]
    CreatingMarketPriceClient(#[from] MarketPriceClientCreateError),
}

async fn real_main() -> Result<(), AggregatorError> {
    let args = App::new("aggregator")
        .arg(logging::build_arg())
        .arg(
            Arg::with_name("redis_url")
                .short("-r")
                .long("--redis-url")
                .env("REDIS_URL")
                .help("URL to connect to Redis with")
                .takes_value(true)
                .default_value("redis://redis:6379"),
        )
        .arg(
            Arg::with_name("csgofloat_key")
                .short("-c")
                .long("--csgofloat-key")
                .env("CSGOFLOAT_KEY")
                .help("API key for CSGOFloat")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("bind_addr")
                .short("-b")
                .long("--bind-addr")
                .env("BIND_ADDR")
                .help("Address to bind server to")
                .takes_value(true)
                .default_value("0.0.0.0:7000"),
        )
        .arg(
            Arg::with_name("keystore_path")
                .index(1)
                .env("KEYSTORE_PATH")
                .help("location of user keystore file")
                .takes_value(true)
                .default_value("./keystore.yaml"),
        )
        .get_matches();

    let log_level = args.value_of(logging::LEVEL_FLAG_NAME).unwrap();
    logging::init_str(log_level);

    let redis_url = args.value_of("redis_url").unwrap();
    let csgofloat_key = args.value_of("csgofloat_key").unwrap();

    let info: ConnectionInfo = redis_url.parse().map_err(AggregatorError::InvalidRedisUrl)?;
    let bind_addr: SocketAddr = args.value_of("bind_addr").unwrap().parse()?;

    let keystore_path = args.value_of("keystore_path").unwrap();
    let keystore = KeyStore::load_from_file(keystore_path).await?;

    let store = Store::new(info.clone()).await?;
    let csgo_float = CsgoFloatClient::new(csgofloat_key, info.clone()).await?;
    let market_price_client = MarketPriceClient::new(info).await?;

    let h = Handler::new(store, keystore, csgo_float, market_price_client);

    serve(&bind_addr, h).await.unwrap();

    Ok(())
}
