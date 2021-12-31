use std::path::{Path, PathBuf};

use clap::{App, Arg};
use reqwest::{Client, Url};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

lazy_static::lazy_static! {
    static ref BINARY_URL: Url = "https://www.dropbox.com/s/x0vxr1kvpadugga/collector.exe".parse().unwrap();
}

#[tokio::main]
async fn main() {
    let args = App::new("bootstrap")
        .arg(logging::build_arg())
        .arg(
            Arg::with_name("config_path")
                .index(1)
                .env("CONFIG_PATH")
                .help("Path to configuration file")
                .takes_value(true)
                .default_value("./config.yaml"),
        )
        .arg(
            Arg::with_name("out_file")
                .short("-o")
                .long("--out-file")
                .help("Path to write the output binary to")
                .takes_value(true)
                .default_value("./collector.exe"),
        )
        .arg(
            Arg::with_name("dry_run")
                .short("-d")
                .long("--dry-run")
                .takes_value(false)
                .help("If provided, don't actually run the binary, just prepare it")
        )
        .get_matches();

    let log_level: String = args.value_of(logging::LEVEL_FLAG_NAME).unwrap().to_string();
    logging::init_str(&log_level);

    let config_path: PathBuf = args.value_of("config_path").unwrap().parse().unwrap();
    if !config_path.exists() {
        log::warn!(
            "Config file {} doesn't exist",
            config_path.to_str().unwrap()
        );

        std::process::exit(1);
    }

    let out_path: PathBuf = args.value_of("out_file").unwrap().parse().unwrap();
    if !out_path.exists() {
        log::info!("Downloading binary to {}...", out_path.as_os_str().to_str().unwrap());
        download_file(&out_path).await;
    }

    let is_dry_run = args.is_present("dry_run");

    if !is_dry_run {
        Command::new(out_path.as_os_str())
            .arg(config_path.as_os_str())
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
