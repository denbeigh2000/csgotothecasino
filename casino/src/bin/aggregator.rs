use std::fmt::{self, Display};
use std::net::{AddrParseError, SocketAddr};

use casino::aggregator::keystore::{KeyStore, KeyStoreLoadSaveError};
use casino::aggregator::{serve, Handler};
use casino::csgofloat::{CsgoFloatClient, CsgoFloatClientCreateError};
use casino::logging;
use casino::steam::{MarketPriceClient, MarketPriceClientCreateError};
use casino::store::{Error as StoreError, Store};
use clap::{App, Arg};
use redis::{ConnectionInfo, RedisError};

#[tokio::main]
async fn main() {
    if let Err(e) = real_main().await {
        log::error!("fatal error: {}", e);
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum Error {
    InvalidBindIP(AddrParseError),
    InvalidRedisUrl(RedisError),
    CreatingCsgoFloatClient(CsgoFloatClientCreateError),
    CreatingStore(StoreError),
    LoadingKeystore(KeyStoreLoadSaveError),
    CreatingMarketPriceClient(MarketPriceClientCreateError),
}

impl From<AddrParseError> for Error {
    fn from(e: AddrParseError) -> Self {
        Self::InvalidBindIP(e)
    }
}

impl From<CsgoFloatClientCreateError> for Error {
    fn from(e: CsgoFloatClientCreateError) -> Self {
        Self::CreatingCsgoFloatClient(e)
    }
}

impl From<StoreError> for Error {
    fn from(e: StoreError) -> Self {
        Self::CreatingStore(e)
    }
}

impl From<KeyStoreLoadSaveError> for Error {
    fn from(e: KeyStoreLoadSaveError) -> Self {
        Self::LoadingKeystore(e)
    }
}

impl From<MarketPriceClientCreateError> for Error {
    fn from(e: MarketPriceClientCreateError) -> Self {
        Self::CreatingMarketPriceClient(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBindIP(e) => write!(f, "invalid bind address given: {}", e),
            Self::InvalidRedisUrl(e) => write!(f, "invalid redis url given: {}", e),
            Self::CreatingCsgoFloatClient(e) => write!(f, "error creating csgofloat client: {}", e),
            Self::CreatingStore(e) => write!(f, "error creating backing store: {}", e),
            Self::LoadingKeystore(e) => write!(f, "error loading keystore: {}", e),
            Self::CreatingMarketPriceClient(e) => {
                write!(f, "error creating steam market price client: {}", e)
            }
        }
    }
}

async fn real_main() -> Result<(), Error> {
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

    let info: ConnectionInfo = redis_url.parse().map_err(Error::InvalidRedisUrl)?;
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
