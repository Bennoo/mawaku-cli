use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Value;

pub const DEFAULT_PROMPT: &str = "A hyper photo realistic unique background for a video call. Don't place me in the frame; the goal is to use the scene as a virtual background in applications like Zoom. Highlight a cosy, lived-in interior with realistic proportions and warm details. Camera height is around eye level when sitting.";
pub const DEFAULT_GEMINI_API_KEY: &str = "";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine configuration directory")]
    ConfigDirUnavailable,
    #[error("failed to read or write configuration file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to deserialize configuration: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("failed to serialize configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub gemini_api_key: String,
    pub image_output_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gemini_api_key: DEFAULT_GEMINI_API_KEY.to_string(),
            image_output_dir: default_image_output_dir().unwrap_or_else(|_| ".".to_string()),
        }
    }
}

/// Loads the Mawaku configuration from disk, creating a default file if absent.
pub fn load_or_init() -> Result<LoadOutcome, ConfigError> {
    let path = config_file_path()?;

    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        let mut value: Value = toml::from_str(&contents)?;
        let mut should_rewrite = false;

        if let Value::Table(ref mut table) = value {
            if table.remove("default_prompt").is_some() {
                should_rewrite = true;
            }
        }

        let is_image_dir_missing_or_invalid = match value.get("image_output_dir") {
            Some(Value::String(value)) => value.trim().is_empty(),
            Some(_) => true,
            None => true,
        };

        let mut config: Config = value.try_into()?;
        let expected_dir = default_image_output_dir_for(&path);

        let empty_field = config.image_output_dir.trim().is_empty();

        if is_image_dir_missing_or_invalid || empty_field {
            config.image_output_dir = expected_dir;
            should_rewrite = true;
        }

        if should_rewrite {
            save(&config, &path)?;
        }

        Ok(LoadOutcome {
            config,
            path,
            created: false,
        })
    } else {
        ensure_parent_exists(&path)?;
        let mut config = Config::default();
        config.image_output_dir = default_image_output_dir_for(&path);
        save(&config, &path)?;
        Ok(LoadOutcome {
            config,
            path,
            created: true,
        })
    }
}

/// Persist the given Mawaku configuration to disk at the provided path.
pub fn save(config: &Config, path: &Path) -> Result<(), ConfigError> {
    ensure_parent_exists(path)?;
    let serialized = toml::to_string_pretty(config)?;
    fs::write(path, serialized)?;
    Ok(())
}

#[derive(Debug)]
pub struct LoadOutcome {
    pub config: Config,
    pub path: PathBuf,
    pub created: bool,
}

fn ensure_parent_exists(path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn config_file_path() -> Result<PathBuf, ConfigError> {
    Ok(config_directory()?.join("config.toml"))
}

fn config_directory() -> Result<PathBuf, ConfigError> {
    let base_dirs = BaseDirs::new().ok_or(ConfigError::ConfigDirUnavailable)?;
    Ok(base_dirs.home_dir().join(".mawaku"))
}

fn default_image_output_dir() -> Result<String, ConfigError> {
    config_directory().map(|path| path.to_string_lossy().into_owned())
}

fn default_image_output_dir_for(path: &Path) -> String {
    path.parent()
        .map(|dir| dir.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

#[cfg(test)]
mod tests;
