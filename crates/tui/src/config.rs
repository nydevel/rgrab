use std::path::PathBuf;

use serde::Deserialize;

const DEFAULT_SERVER_NAME: &str = "local";
const DEFAULT_SERVER_URL: &str = "http://localhost:3000";
const CONFIG_DIR_NAME: &str = ".rgrab";
const CONFIG_FILE_NAME: &str = "config.toml";

const DEFAULT_CONFIG_CONTENT: &str = "\
# rgrab TUI configuration

[[servers]]
name = \"local\"
url = \"http://localhost:3000\"
";

#[derive(Deserialize, Default)]
struct FileConfig {
    servers: Option<Vec<ServerEntry>>,
}

#[derive(Deserialize, Clone)]
pub struct ServerEntry {
    pub name: String,
    pub url: String,
}

pub struct Config {
    pub servers: Vec<ServerEntry>,
}

pub fn load(cli_server: Option<String>) -> Config {
    ensure_default_config();
    let file_cfg = load_file_config();
    let mut servers = build_server_list(file_cfg);
    if let Some(url) = cli_server {
        servers.insert(
            0,
            ServerEntry {
                name: "cli".to_string(),
                url,
            },
        );
    }
    Config { servers }
}

fn build_server_list(file_cfg: FileConfig) -> Vec<ServerEntry> {
    if let Some(servers) = file_cfg.servers
        && !servers.is_empty()
    {
        return servers;
    }
    vec![ServerEntry {
        name: DEFAULT_SERVER_NAME.to_string(),
        url: DEFAULT_SERVER_URL.to_string(),
    }]
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
