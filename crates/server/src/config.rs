use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

const DEFAULT_DATA_DIR: &str = "./data/rgrab";
const DEFAULT_LISTEN: &str = "0.0.0.0:3000";
const DEFAULT_LOG_LEVEL: &str = "info";

#[derive(Parser)]
#[command(name = "rgrab", about = "Lightweight observability backend")]
struct Cli {
    /// Path to TOML config file
    #[arg(short, long, default_value = "/etc/rgrab/rgrab.toml")]
    config: PathBuf,

    /// Data directory for RocksDB storage
    #[arg(short, long)]
    data_dir: Option<String>,

    /// Listen address (host:port)
    #[arg(short, long)]
    listen: Option<String>,

    /// Log level filter (trace, debug, info, warn, error)
    #[arg(long)]
    log_level: Option<String>,
}

#[derive(Deserialize, Default)]
struct FileConfig {
    data_dir: Option<String>,
    listen: Option<String>,
    log_level: Option<String>,
}

pub struct Config {
    pub data_dir: String,
    pub listen: String,
    pub log_level: String,
}

impl Config {
    pub fn load() -> Self {
        let cli = Cli::parse();

        let file_cfg = load_file_config(&cli.config);

        let data_dir = cli
            .data_dir
            .or(file_cfg.data_dir)
            .unwrap_or_else(|| DEFAULT_DATA_DIR.to_string());

        let listen = cli
            .listen
            .or(file_cfg.listen)
            .unwrap_or_else(|| DEFAULT_LISTEN.to_string());

        let log_level = cli
            .log_level
            .or(file_cfg.log_level)
            .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_string());

        Self {
            data_dir,
            listen,
            log_level,
        }
    }
}

fn load_file_config(path: &PathBuf) -> FileConfig {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return FileConfig::default();
    };

    match toml::from_str(&contents) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: failed to parse config {}: {e}", path.display());
            FileConfig::default()
        }
    }
}
