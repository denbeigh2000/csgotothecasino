use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

pub static LEVEL_FLAG_NAME: &str = "log_level";

pub fn init_str(level: &str) {
    init(get_level(level));
}

pub fn init(level: LevelFilter) {
    let log_config = ConfigBuilder::new()
        .set_target_level(level)
        .set_max_level(LevelFilter::Info)
        .set_time_offset_to_local()
        .unwrap()
        .build();

    TermLogger::init(
        LevelFilter::Info,
        log_config,
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )
    .unwrap();
}

pub fn get_level(given: &str) -> LevelFilter {
    match given {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => unreachable!(),
    }
}
