use std::path::PathBuf;

use serde::Deserialize;

const DEFAULT_SERVER: &str = "http://localhost:3000";
const CONFIG_DIR_NAME: &str = ".rgrab";
const CONFIG_FILE_NAME: &str = "config.toml";

const DEFAULT_CONFIG_CONTENT: &str = "\
# rgrab TUI configuration
server = \"http://localhost:3000\"
";

#[derive(Deserialize, Default)]
struct FileConfig {
    server: Option<String>,
}

pub struct Config {
    pub server: String,
}

pub fn load(cli_server: Option<String>) -> Config {
    ensure_default_config();
    let file_cfg = load_file_config();
    Config {
        server: cli_server
            .or(file_cfg.server)
            .unwrap_or_else(|| DEFAULT_SERVER.to_string()),
    }
}

fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(CONFIG_DIR_NAME))
}

fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(CONFIG_FILE_NAME))
}

fn ensure_default_config() {
    let Some(dir) = config_dir() else {
        return;
    };
    let path = dir.join(CONFIG_FILE_NAME);
    if path.exists() {
        return;
    }
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(&path, DEFAULT_CONFIG_CONTENT);
}

fn load_file_config() -> FileConfig {
    let Some(path) = config_path() else {
        return FileConfig::default();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return FileConfig::default();
    };
    toml::from_str(&contents).unwrap_or_default()
}
