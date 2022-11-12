use std::path::{Path, PathBuf};

use clap::Parser;
use reqwest::{Client, Url};
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

#[tokio::main]
async fn main() {
    // TODO
    let args = Args::parse();

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

    if !args.out_file.exists() {
        log::info!("Downloading binary to {}...", args.out_file.as_os_str().to_str().unwrap());
        download_file(&args.out_file).await;
    }

    if !args.dry_run {
        Command::new(args.out_file.as_os_str())
            .arg(args.config_path.as_os_str())
            .spawn()
            .unwrap()
            .wait()
            .await
            .unwrap();
    }
}

async fn download_file(out: &Path) {
    let client = Client::new();

    let resp = client.get(BINARY_URL.clone()).send().await.unwrap();
    let bstream = resp.bytes().await.unwrap();
    let mut f = File::create(out).await.unwrap();

    f.write_all(&bstream).await.unwrap();
}
