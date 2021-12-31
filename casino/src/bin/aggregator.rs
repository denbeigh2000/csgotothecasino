use std::env::VarError;
use std::fmt::Display;
use std::net::{AddrParseError, SocketAddr};
use std::{env, fmt};

use bb8_redis::redis::{ConnectionInfo, RedisError};
use casino::aggregator::keystore::{KeyStore, KeyStoreLoadSaveError};
use casino::aggregator::{serve, Handler};
use casino::csgofloat::{CsgoFloatClient, CsgoFloatClientCreateError};
use casino::steam::{MarketPriceClient, MarketPriceClientCreateError};
use casino::store::{Error as StoreError, Store};
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

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
    NonUnicodeBindAddr,
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
            Self::NonUnicodeBindAddr => write!(f, "non-unicode bind addr given"),
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
    let log_config = ConfigBuilder::new()
        .set_target_level(LevelFilter::Info)
        .set_max_level(LevelFilter::Info)
        .set_time_to_local(true)
        .build();
    TermLogger::init(
        LevelFilter::Info,
        log_config,
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )
    .unwrap();

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL unset");
    let csgofloat_key = env::var("CSGOFLOAT_KEY").expect("CSGOFLOAT_KEY unset");
    let info: ConnectionInfo = redis_url.parse().map_err(Error::InvalidRedisUrl)?;

    let bind_addr: SocketAddr = env::var("BIND_ADDR")
        .map(Some)
        .or_else(|e| match e {
            VarError::NotPresent => Ok(None),
            VarError::NotUnicode(_) => Err(Error::NonUnicodeBindAddr),
        })?
        .map(|a| a.parse())
        .transpose()?
        .unwrap_or_else(|| ([0, 0, 0, 0], 7000).into());

    let store = Store::new(info.clone()).await?;
    let keystore = KeyStore::load_from_file("./keystore.yaml").await?;
    let csgo_float = CsgoFloatClient::new(csgofloat_key, info.clone()).await?;
    let market_price_client = MarketPriceClient::new(info).await?;

    let h = Handler::new(store, keystore, csgo_float, market_price_client);

    serve(&bind_addr, h).await.unwrap();

    Ok(())
}
