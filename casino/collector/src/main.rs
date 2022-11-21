use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::time::Duration;

use collector::config::{Config, ConfigLoadError};
use collector::{Collector, CollectorError, UrlParseError};
use chrono::Utc;
use clap::Parser;
use reqwest::Url;
use steam::errors::AuthenticationCheckError;
use steam::{CredentialParseError, Id, IdUrlParseError, SteamClient, SteamCredentials};
use thiserror::Error;
use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    if let Err(e) = main_result().await {
        log::error!("fatal error: {}", e);
        std::process::exit(1);
    }
}

#[derive(Parser)]
struct Args {
    /// Interval to poll Steam API
    #[arg(short, long, env, default_value = "60s")]
    poll_interval: humantime::Duration,
    /// URL to upload entries to
    #[arg(short, long, env, default_value = "https://casino.denb.ee/api/upload")]
    collection_url: Url,
    /// Path to credentials storage file
    #[arg(short = 'x', long, env, default_value = "./.creds.json")]
    credentials_path: PathBuf,
    /// Path to configuration file
    config_path: PathBuf,
    /// Level to log at
    #[arg(short, long, env, default_value = "info")]
    log_level: log::LevelFilter,
}

async fn main_result() -> Result<(), MainError> {
    let args = Args::parse();

    logging::init(args.log_level);

    let cfg = Config::try_from_path(args.config_path).await?;

    let id = Id::try_from_url(&cfg.steam_profile_url).await?;

    let client = prepare_client(id, AsRef::as_ref(&args.credentials_path)).await?;

    // TODO: Can we get the current time/time zone from Steam, so that we can
    // avoid deltas with local time and store timezone?
    let now = Utc::now();
    let delta = chrono::Duration::from_std(Duration::from_secs(60 * 10)).unwrap();
    let start = now - delta;
    let st = Some(start);

    Collector::new(args.collection_url, client, cfg.pre_shared_key, *args.poll_interval, st)
        .await?
        .run()
        .await?;

    Ok(())
}

#[derive(Debug, Error)]
enum MainError {
    #[error("error loading config: {0}")]
    LoadingConfig(#[from] ConfigLoadError),
    #[error("error gathering user credentials: {0}")]
    PreparingClient(#[from] ClientPrepareError),
    #[error("error gathering steam id info: {0}")]
    GatheringSteamIdInfo(#[from] IdUrlParseError),
    #[error("error parsing collection url: {0}")]
    CollectionUrlParse(#[from] UrlParseError),
    #[error("error parsing polling interval: {0}")]
    InvalidIntervalValue(#[from] ParseIntError),
    #[error("error running main loop: {0}")]
    RunningCollector(#[from] CollectorError),
}


#[derive(Debug, Error)]
enum ClientPrepareError {
    #[error("io error: {0}")]
    IO(#[from] io::Error),
    #[error("error prompting for secrets: {0}")]
    Prompt(#[from] CredentialPromptError),
    #[error("error checking for authentication: {0}")]
    AuthCheck(#[from] AuthenticationCheckError),
}


async fn prepare_client(id: Id, creds_path: &Path) -> Result<SteamClient, ClientPrepareError> {
    if creds_path.exists() {
        let creds = match load_credentials_from_file(creds_path).await {
            Ok(creds) => Some(creds),
            Err(CredentialLoadSaveError::IO(e)) => return Err(e.into()),
            Err(CredentialLoadSaveError::Parse(e)) => {
                log::warn!("error parsing credentials: {}", e);
                fs::remove_file(creds_path).await?;
                None
            }
        };

        if let Some(creds) = creds {
            let client = SteamClient::new(id.clone(), creds);
            if client.is_authenticated().await? {
                return Ok(client);
            }
        }
    }

    loop {
        let creds = match prompt_for_credentials().await {
            Ok(creds) => creds,
            Err(CredentialPromptError::CredentialParse(e)) => {
                log::warn!("error parsing cookie: {}", e);
                continue;
            }
            Err(CredentialPromptError::IO(e)) => return Err(e.into()),
        };

        let client = SteamClient::new(id.clone(), creds.clone());
        if !client.is_authenticated().await? {
            log::warn!("authentication unsuccessful");
            continue;
        }

        log::info!("authentication successful");
        if let Err(e) = save_credentials_to_file(creds_path, &creds).await {
            log::warn!("error saving credentials to file: {}", e);
            log::warn!("continuing without saving, you will need to enter these again next time");
        }

        return Ok(client);
    }
}

#[derive(Debug, Error)]
enum CredentialPromptError {
    #[error("error parsing credentials: {0}")]
    CredentialParse(#[from] CredentialParseError),
    #[error("io error: {0}")]
    IO(#[from] io::Error),
}

async fn prompt_for_credentials() -> Result<SteamCredentials, CredentialPromptError> {
    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = io::stdout();

    stdout.flush().await?;
    stdout
        .write_all("Please enter steam cookie:\n>".as_bytes())
        .await?;
    let mut buf = String::new();
    stdin.read_line(&mut buf).await?;

    let creds = SteamCredentials::try_from_cookie_str(buf.as_str())?;

    Ok(creds)
}

#[derive(Debug, Error)]
enum CredentialLoadSaveError {
    #[error("error parsing json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("io error: {0}")]
    IO(#[from] io::Error),
}

async fn load_credentials_from_file(p: &Path) -> Result<SteamCredentials, CredentialLoadSaveError> {
    let mut f = fs::File::open(p).await?;
    let mut buf: Vec<u8> = Vec::new();
    f.read_to_end(&mut buf).await?;
    let parsed = serde_json::from_slice(&buf)?;

    Ok(parsed)
}

async fn save_credentials_to_file(
    p: &Path,
    creds: &SteamCredentials,
) -> Result<(), CredentialLoadSaveError> {
    let encoded = serde_json::to_vec(creds)?;
    let mut f = fs::File::create(p).await?;
    f.write_all(&encoded).await?;

    Ok(())
}
