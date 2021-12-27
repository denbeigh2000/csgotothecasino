use chrono::{NaiveDate, TimeZone, Utc};
use std::path::{Path, PathBuf};

use tokio::fs;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use casino::collector::config::Config;
use casino::collector::Collector;
use casino::steam::{CredentialParseError, SteamCredentials};

lazy_static::lazy_static! {
    static ref CREDS_PATH: PathBuf = PathBuf::from("./.creds.json");
}

#[tokio::main]
async fn main() {
    let config = Config::try_from_path("config.yaml").await.unwrap();
    let steam_creds = if CREDS_PATH.exists() {
        load_credentials_from_file(CREDS_PATH.as_path())
            .await
            .unwrap()
    } else {
        let creds = prompt_for_credentials().await.unwrap();
        save_credentials_to_file(&CREDS_PATH, &creds).await.unwrap();
        creds
    };

    let naive_start_time = NaiveDate::from_ymd(2021, 11, 21).and_hms(0, 0, 0);
    let start_time = Utc.from_local_datetime(&naive_start_time).unwrap();

    Collector::from_config(config, steam_creds, Some(start_time))
        .await
        .unwrap()
        .run()
        .await
        .unwrap();
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
