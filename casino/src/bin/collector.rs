use casino::steam::errors::AuthenticationCheckError;
use chrono::{NaiveDate, TimeZone, Utc};
use clap::{App, Arg};
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use casino::collector::config::Config;
use casino::collector::Collector;
use casino::steam::{ClientCreateError, CredentialParseError, SteamClient, SteamCredentials};

lazy_static::lazy_static! {
    static ref CREDS_PATH: PathBuf = PathBuf::from("./.creds.json");
}

#[tokio::main]
async fn main() {
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

    let cfg_path = args.value_of("config").unwrap();
    let cfg = Config::try_from_path(cfg_path).await.unwrap();

    let client = prepare_client(&cfg.steam_username).await.unwrap();

    let interval_secs = args.value_of("interval").unwrap().parse().unwrap();
    let interval = Duration::from_secs(interval_secs);

    let naive_start_time = NaiveDate::from_ymd(2021, 11, 21).and_hms(0, 0, 0);
    let start_time = Utc.from_local_datetime(&naive_start_time).unwrap();
    let st = Some(start_time);

    Collector::new(client, cfg.pre_shared_key, interval, st)
        .await
        .unwrap()
        .run()
        .await
        .unwrap();
}

#[derive(Debug)]
enum ClientPrepareError {
    IO(io::Error),
    Prompt(CredentialPromptError),
    AuthCheck(AuthenticationCheckError),
    ClientCreate(ClientCreateError),
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

impl From<ClientCreateError> for ClientPrepareError {
    fn from(e: ClientCreateError) -> Self {
        ClientPrepareError::ClientCreate(e)
    }
}

async fn prepare_client(steam_username: &str) -> Result<SteamClient, ClientPrepareError> {
    if CREDS_PATH.exists() {
        let path = CREDS_PATH.as_path();
        let creds = match load_credentials_from_file(path).await {
            Ok(creds) => Some(creds),
            Err(CredentialLoadSaveError::IOError(e)) => return Err(e.into()),
            Err(CredentialLoadSaveError::ParseError(e)) => {
                eprintln!("error parsing credentials: {}", e);
                fs::remove_file(path).await?;
                None
            }
        };

        if let Some(creds) = creds {
            let client = SteamClient::new(steam_username.to_string(), creds).await?;
            if client.is_authenticated().await? {
                return Ok(client);
            }
        }
    }

    loop {
        let creds = prompt_for_credentials().await?;
        let client = SteamClient::new(steam_username.to_string(), creds.clone()).await?;
        if !client.is_authenticated().await? {
            eprintln!("authentication unsuccessful");
            continue;
        }

        eprintln!("authentication successful");
        if let Err(e) = save_credentials_to_file(&CREDS_PATH, &creds).await {
            eprintln!("error saving credentials to file: {:?}", e);
            eprintln!("continuing without saving, you will need to enter these again next time");
        }

        return Ok(client);
    }
}

#[derive(Debug)]
enum CredentialPromptError {
    CredentialParseError(CredentialParseError),
    IOError(io::Error),
}

impl From<CredentialParseError> for CredentialPromptError {
    fn from(e: CredentialParseError) -> Self {
        Self::CredentialParseError(e)
    }
}

impl From<io::Error> for CredentialPromptError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
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
    ParseError(serde_json::Error),
    IOError(io::Error),
}

impl From<io::Error> for CredentialLoadSaveError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<serde_json::Error> for CredentialLoadSaveError {
    fn from(e: serde_json::Error) -> Self {
        Self::ParseError(e)
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
