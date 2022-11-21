use std::path::{Path, PathBuf};

use clap::Parser;
use reqwest::{Client, Url};
use thiserror::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

lazy_static::lazy_static! {
    static ref BINARY_URL: Url = "https://www.dropbox.com/s/x0vxr1kvpadugga/collector.exe".parse().unwrap();
}

#[derive(Parser)]
struct Args {
    /// Path to configuration file
    #[arg(env, default_value = "./config.yaml")]
    config_path: PathBuf,
    /// Path to write the output binary to
    #[arg(short, long, env, default_value = "./collector.exe")]
    out_file: PathBuf,
    /// If provided, don't actually run the binary, just prepare it
    #[arg(short, long, env, action = clap::ArgAction::SetTrue)]
    dry_run: bool,
    /// Level to log at
    #[arg(short, long, env, default_value = "info")]
    log_level: log::LevelFilter,
}

#[derive(Debug, Error)]
enum BootstrapError {
    #[error("{0}")]
    ParsingCommandLineArgs(#[from] clap::Error),
    #[error("Config path does not exist at {0}")]
    ConfigDoesNotExist(PathBuf),
    #[error("Error downloading binary ({0})")]
    DownloadingBinary(#[from] DownloadError),
    #[error("Error running end binary ({0})")]
    RunningBinary(#[from] std::io::Error),
}

#[derive(Debug, Error)]
enum DownloadError {
    #[error("failed to download file ({0})")]
    Transport(#[from] reqwest::Error),
    #[error("io error when writing file")]
    IO(#[from] std::io::Error),
}

#[tokio::main]
async fn main() {
    match inner_main().await {
        Ok(()) => return,
        Err(BootstrapError::ParsingCommandLineArgs(e)) => eprintln!("{e}"),
        Err(e) => log::error!("{e}"),
    }

    std::process::exit(1);
}

async fn inner_main() -> Result<(), BootstrapError> {
    // TODO
    let args = Args::try_parse()?;

    // let log_level: String = args.value_of(logging::LEVEL_FLAG_NAME).unwrap().to_string();
    logging::init(args.log_level);

    // let config_path: PathBuf = args.value_of("config_path").unwrap().parse().unwrap();
    if !args.config_path.exists() {
        log::warn!(
            "Config file {} doesn't exist",
            args.config_path.to_str().unwrap()
        );

        std::process::exit(1);
    }

    if !args.config_path.exists() {
        return Err(BootstrapError::ConfigDoesNotExist(args.config_path));
    }

    if !args.out_file.exists() {
        log::info!(
            "Downloading binary to {}...",
            args.out_file.as_os_str().to_str().unwrap()
        );
        download_file(&args.out_file).await?;
    }

    if !args.dry_run {
        Command::new(args.out_file.as_os_str())
            .arg(args.config_path.as_os_str())
            .spawn()?
            .wait().await.unwrap();
    }

    Ok(())
}

async fn download_file(out: &Path) -> Result<(), DownloadError> {
    let client = Client::new();

    let resp = client.get(BINARY_URL.clone()).send().await?;
    let bstream = resp.bytes().await?;
    let mut f = File::create(out).await?;

    f.write_all(&bstream).await?;

    Ok(())
}
