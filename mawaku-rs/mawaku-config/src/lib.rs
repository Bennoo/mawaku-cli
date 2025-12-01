use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Value;

pub const DEFAULT_PROMPT: &str = "\
Photo, hyper-photorealistic, 4K, HDR, studio lighting, indistinguishable from a real photo. \
Strictly avoid synthetic, CGI, or video-game-style visuals. \
Use a wide-angle lens with minimal distortion so proportions stay natural and avoid any fisheye warping. \
Show a very spacious room with generous depth, cosy modern details, and believable scale. \
Keep the foreground empty never include monitors, screens, desk edges, or camera equipment in the close-up. \
Don't place any people or body parts in the frame, and avoid smoke in the scene. \
Present an unobstructed and clearview of the room with a large modern window revealing a beautiful outdoor scene. \
Camera angle: professional webcam-style vantage facing into the room while hovering just in front of the desk so no furniture crosses the frame edge. \
Camera height: slightly above seated eye level, matching a real highly positioned webcam’s perspective. \
Camera location: prefer a corner vantage that reveals depth. \
The scene should feel like the believable background behind someone on a video call.";
pub const DEFAULT_GEMINI_API_KEY_ENV_VAR: &str = "GEMINI_API_KEY";

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
    pub gemini_api: GeminiApiConfig,
    /// Stored at the root of `config.toml` for backward compatibility with
    /// earlier Mawaku versions that only understood this top-level key.
    pub image_output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeminiApiConfig {
    pub api_key_env_var: String,
}

impl GeminiApiConfig {
    pub fn api_key_env_var(&self) -> &str {
        if self.api_key_env_var.trim().is_empty() {
            DEFAULT_GEMINI_API_KEY_ENV_VAR
        } else {
            self.api_key_env_var.as_str()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gemini_api: GeminiApiConfig::default(),
            image_output_dir: default_image_output_dir().unwrap_or_else(|_| ".".to_string()),
        }
    }
}

impl Default for GeminiApiConfig {
    fn default() -> Self {
        Self {
            api_key_env_var: DEFAULT_GEMINI_API_KEY_ENV_VAR.to_string(),
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

            if table.remove("gemini_api_key").is_some() {
                should_rewrite = true;
            }

            if let Some(Value::Table(gemini_api)) = table.get_mut("gemini_api") {
                let mut updated_env_var = None;
                if !gemini_api.contains_key("api_key_env_var")
                    && let Some(env_var) = gemini_api
                        .get("environment")
                        .and_then(Value::as_str)
                        .and_then(|environment| {
                            gemini_api
                                .get("environments")
                                .and_then(Value::as_table)
                                .and_then(|environments| {
                                    environments.get(environment).and_then(Value::as_str)
                                })
                        })
                {
                    updated_env_var = Some(env_var.to_string());
                }

                if gemini_api.remove("environment").is_some() {
                    should_rewrite = true;
                }

                if gemini_api.remove("environments").is_some() {
                    should_rewrite = true;
                }

                if !gemini_api.contains_key("api_key_env_var") {
                    let value = updated_env_var
                        .unwrap_or_else(|| DEFAULT_GEMINI_API_KEY_ENV_VAR.to_string());
                    gemini_api.insert("api_key_env_var".to_string(), Value::String(value));
                    should_rewrite = true;
                }
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
        let config = Config {
            image_output_dir: default_image_output_dir_for(&path),
            ..Config::default()
        };
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
