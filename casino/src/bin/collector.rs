use std::fmt::{self, Display};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::time::Duration;

use casino::collector::config::{Config, ConfigLoadError};
use casino::collector::{Collector, CollectorError};
use casino::steam::errors::AuthenticationCheckError;
use casino::steam::{CredentialParseError, Id, IdUrlParseError, SteamClient, SteamCredentials};
use chrono::{NaiveDate, TimeZone, Utc};
use clap::{App, Arg};
use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

lazy_static::lazy_static! {
    static ref CREDS_PATH: PathBuf = PathBuf::from("./.creds.json");
}

#[tokio::main]
async fn main() {
    if let Err(e) = main_result().await {
        log::error!("fatal error: {}", e);
        std::process::exit(1);
    }
}

async fn main_result() -> Result<(), Error> {
    let args = App::new("collector")
        .arg(
            Arg::with_name("interval")
                .short("-i")
                .long("--interval-secs")
                .help("Interval to poll Steam API")
                .takes_value(true)
                .default_value("30"),
        )
        .arg(
            Arg::with_name("config")
                .help("Path to configuration file")
                .takes_value(true)
                .default_value("./config.yaml")
                .index(1),
        )
        .get_matches();

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

    let cfg_path = args.value_of("config").ok_or(Error::NoConfigValue)?;
    let cfg = Config::try_from_path(cfg_path).await?;

    let id = Id::try_from_url(&cfg.steam_profile_url).await?;

    let client = prepare_client(id).await?;

    let interval_secs = args
        .value_of("interval")
        .ok_or(Error::NoIntervalValue)?
        .parse()
        .map_err(Error::InvalidIntervalValue)?;
    let interval = Duration::from_secs(interval_secs);

    let naive_start_time = NaiveDate::from_ymd(2021, 11, 21).and_hms(0, 0, 0);
    let start_time = Utc.from_local_datetime(&naive_start_time).unwrap();
    let st = Some(start_time);

    Collector::new(client, cfg.pre_shared_key, interval, st)
        .await
        .run()
        .await?;

    Ok(())
}

#[derive(Debug)]
enum Error {
    LoadingConfig(ConfigLoadError),
    PreparingClient(ClientPrepareError),
    GatheringSteamIdInfo(IdUrlParseError),
    NoConfigValue,
    NoIntervalValue,
    InvalidIntervalValue(ParseIntError),
    RunningCollector(CollectorError),
}

impl From<ConfigLoadError> for Error {
    fn from(e: ConfigLoadError) -> Self {
        Self::LoadingConfig(e)
    }
}

impl From<ClientPrepareError> for Error {
    fn from(e: ClientPrepareError) -> Self {
        Self::PreparingClient(e)
    }
}

impl From<CollectorError> for Error {
    fn from(e: CollectorError) -> Self {
        Self::RunningCollector(e)
    }
}

impl From<IdUrlParseError> for Error {
    fn from(e: IdUrlParseError) -> Self {
        Self::GatheringSteamIdInfo(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadingConfig(e) => write!(f, "error loading config: {}", e),
            Self::PreparingClient(e) => write!(f, "error gathering user credentials: {}", e),
            Self::GatheringSteamIdInfo(e) => write!(f, "error gathering steam id info: {}", e),
            Self::NoConfigValue => write!(f, "no value found for config path"),
            Self::NoIntervalValue => write!(f, "no value found for interval"),
            Self::InvalidIntervalValue(e) => write!(f, "error parsing polling interval: {}", e),
            Self::RunningCollector(e) => write!(f, "error running main loop: {}", e),
        }
    }
}

#[derive(Debug)]
enum ClientPrepareError {
    IO(io::Error),
    Prompt(CredentialPromptError),
    AuthCheck(AuthenticationCheckError),
}

impl From<io::Error> for ClientPrepareError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<CredentialPromptError> for ClientPrepareError {
    fn from(e: CredentialPromptError) -> Self {
        Self::Prompt(e)
    }
}

impl From<AuthenticationCheckError> for ClientPrepareError {
    fn from(e: AuthenticationCheckError) -> Self {
        Self::AuthCheck(e)
    }
}

impl Display for ClientPrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO(e) => write!(f, "io error: {}", e),
            Self::Prompt(e) => write!(f, "error prompting for secrets: {}", e),
            Self::AuthCheck(e) => write!(f, "error checking for authentication: {}", e),
        }
    }
}

async fn prepare_client(id: Id) -> Result<SteamClient, ClientPrepareError> {
    if CREDS_PATH.exists() {
        let path = CREDS_PATH.as_path();
        let creds = match load_credentials_from_file(path).await {
            Ok(creds) => Some(creds),
            Err(CredentialLoadSaveError::IO(e)) => return Err(e.into()),
            Err(CredentialLoadSaveError::Parse(e)) => {
                log::warn!("error parsing credentials: {}", e);
                fs::remove_file(path).await?;
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
        if let Err(e) = save_credentials_to_file(&CREDS_PATH, &creds).await {
            log::warn!("error saving credentials to file: {}", e);
            log::warn!("continuing without saving, you will need to enter these again next time");
        }

        return Ok(client);
    }
}

#[derive(Debug)]
enum CredentialPromptError {
    CredentialParse(CredentialParseError),
    IO(io::Error),
}

impl From<CredentialParseError> for CredentialPromptError {
    fn from(e: CredentialParseError) -> Self {
        Self::CredentialParse(e)
    }
}

impl From<io::Error> for CredentialPromptError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl Display for CredentialPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialPromptError::CredentialParse(e) => {
                write!(f, "error parsing credentials: {}", e)
            }
            CredentialPromptError::IO(e) => write!(f, "io error: {}", e),
        }
    }
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

#[derive(Debug)]
enum CredentialLoadSaveError {
    Parse(serde_json::Error),
    IO(io::Error),
}

impl Display for CredentialLoadSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialLoadSaveError::Parse(e) => write!(f, "error parsing json: {}", e),
            CredentialLoadSaveError::IO(e) => write!(f, "io error: {}", e),
        }
    }
}

impl From<io::Error> for CredentialLoadSaveError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<serde_json::Error> for CredentialLoadSaveError {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e)
    }
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
