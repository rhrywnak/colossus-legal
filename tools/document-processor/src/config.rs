//! Configuration loading and validation for the document processor.
//
// Search order (highest priority first):
// 1. CLI flag: --config /path/to/config.toml
// 2. Environment variable: COLOSSUS_CONFIG
// 3. ~/.config/colossus-legal/config.toml
// 4. /etc/colossus-legal/config.toml

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Configuration loaded from config.toml
#[derive(Debug, Deserialize)]
pub struct Config {
    pub ollama: OllamaConfig,
    pub directories: DirectoriesConfig,
    pub defaults: DefaultsConfig,
    pub neo4j: Neo4jConfig,
}

#[derive(Debug, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
    pub temperature: f32,
    pub num_predict: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct DirectoriesConfig {
    pub input_directory: String,
    pub output_directory: String,
    pub prompt_directory: String,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    pub prompt_template: String,
    pub output_suffix: String,
}

#[derive(Debug, Deserialize)]
pub struct Neo4jConfig {
    pub url: String,
    pub user: String,
    pub password: String,
}

/// Load configuration using standard search order.
///
/// Returns (Config, PathBuf) where PathBuf is the config file actually used.
pub fn load_config_with_search(explicit_path: Option<&str>) -> Result<(Config, PathBuf)> {
    let config_path = if let Some(path) = explicit_path {
        let p = PathBuf::from(path);
        if !p.exists() {
            bail!("Config file not found: {}", p.display());
        }
        p
    } else if let Ok(path) = env::var("COLOSSUS_CONFIG") {
        let p = PathBuf::from(path);
        if !p.exists() {
            bail!("Config file not found (from COLOSSUS_CONFIG): {}", p.display());
        }
        p
    } else {
        let home = env::var("HOME").context("HOME environment variable not set")?;
        let user_config = Path::new(&home).join(".config/colossus-legal/config.toml");

        if user_config.exists() {
            user_config
        } else {
            let system_config = PathBuf::from("/etc/colossus-legal/config.toml");
            if system_config.exists() {
                system_config
            } else {
                bail!(
                    "Config file not found!\n\n\
                     Searched:\n\
                     - {}\n\
                     - /etc/colossus-legal/config.toml\n\n\
                     Create config at: ~/.config/colossus-legal/config.toml",
                    user_config.display()
                );
            }
        }
    };

    let config = load_config(&config_path)?;
    Ok((config, config_path))
}

/// Load and parse the TOML config from a specific path.
fn load_config(path: &Path) -> Result<Config> {
    let config_text =
        fs::read_to_string(path).with_context(|| format!("Failed to read config: {}", path.display()))?;

    let config: Config = toml::from_str(&config_text)
        .with_context(|| format!("Failed to parse config: {}", path.display()))?;

    Ok(config)
}
