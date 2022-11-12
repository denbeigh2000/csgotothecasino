use std::net::{AddrParseError, SocketAddr};
use std::path::PathBuf;

use csgofloat::{CsgoFloatClient, CsgoFloatClientCreateError};
use steam::{MarketPriceClient, MarketPriceClientCreateError};
use store::{StoreError as StoreError, Store};
use clap::Parser;
use redis::ConnectionInfo;
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
    #[error("error creating csgofloat client: {0}")]
    CreatingCsgoFloatClient(#[from] CsgoFloatClientCreateError),
    #[error("error creating backing store: {0}")]
    CreatingStore(#[from] StoreError),
    #[error("error loading keystore: {0}")]
    LoadingKeystore(#[from] KeyStoreLoadSaveError),
    #[error("error creating steam market price client: {0}")]
    CreatingMarketPriceClient(#[from] MarketPriceClientCreateError),
}

#[derive(Parser)]
#[command(version)]
struct Args  {
    /// URL to connect to Redis with"
    #[arg(short, long, env, default_value = "redis://redis:6379")]
    redis_url: ConnectionInfo,
    /// API key for CSGOFloat
    #[arg(short, long, env)]
    csgofloat_key: String,
    /// Address to bind server to
    #[arg(short, long, env, default_value = "0.0.0.0:7000")]
    bind_addr: SocketAddr,
    /// Location of user keystore file
    #[arg(short, long, env, default_value = "./keystore.yaml")]
    keystore_path: PathBuf,
    /// Level to log at
    #[arg(short, long, env, default_value = "info")]
    log_level: log::LevelFilter,
}

async fn real_main() -> Result<(), AggregatorError> {
    let args = Args::parse();

    logging::init(args.log_level);

    let keystore = KeyStore::load_from_file(args.keystore_path).await?;
    let store = Store::new(args.redis_url.clone()).await?;
    let csgo_float = CsgoFloatClient::new(args.csgofloat_key, args.redis_url.clone()).await?;
    let market_price_client = MarketPriceClient::new(args.redis_url).await?;

    let h = Handler::new(store, keystore, csgo_float, market_price_client);

    serve(&args.bind_addr, h).await.unwrap();

    Ok(())
}
