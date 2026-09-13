use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Value;

pub const DEFAULT_PROMPT: &str = "\
Create a single 16:9 photographic background for a video call: a real lived-in room seen by a good webcam, temporarily without its occupant. \
The result must have convincing scale, materials and light, and feel like a beautiful home the viewer would love to step into and spend an afternoon in.\n\n\
Camera and framing: the webcam is mounted on a monitor at a desk near one side of the room, about 1.25 metres above the floor. \
It faces past the absent seated caller into the room behind them; the desk and monitor are behind the camera and out of view. \
Keep the camera level, with a natural moderately wide field of view, approximately a 28-35 mm full-frame equivalent lens. \
Use straight verticals and normal furniture proportions, without fisheye or stretched corners. \
Keep the central area where a caller's head and shoulders will appear visually quiet; place distinctive furnishings to either side.\n\n\
Space and layout: show a generously sized but plausible home living room or study, with the back wall roughly 4-6 metres away. \
Leave open floor between the camera and the furniture. Use a few furnishings in the middle distance and a farther wall or open doorway to establish depth. \
Let two walls meet off-centre rather than using a perfectly symmetrical composition. \
Avoid close-up furniture, an empty echoing hall, or an exaggerated luxury showroom. \
Make the room welcoming through a comfortable seating area to one side, tactile natural materials and a harmonious palette with a little local colour. \
Choose a few compatible details, such as a soft linen sofa, a well-made wooden chair, a textured rug or a healthy plant; leave generous breathing room between them. \
Create one appealing place to linger, such as a reading seat beside the window, with a clear path toward it. \
The appeal should come from comfort, thoughtful proportions and personal character, with quality furnishings that look used and cared for.\n\n\
Light and photographic character: follow the requested time of day consistently indoors and outside. \
Use plausible soft window light from the side and practical room lamps where appropriate. \
At dusk, show subdued cool exterior light and restrained warm indoor lamps; at night, rely on indoor lighting and a dark exterior. \
Keep the room readable with natural exposure, gentle shadows, neutral colours and restrained contrast. \
A side window may offer an enticing but geographically plausible glimpse of greenery, a courtyard or the local landscape; keep the room the main subject and avoid a bright backlit rectangle. \
Render subtle fabric texture, wood grain, matte paint, slight wear and small everyday irregularities. \
Keep the background naturally in focus, without portrait blur, artificial sharpening, HDR halos, dramatic sunbeams or cinematic colour grading.\n\n\
Finish: a tidy but inhabited room with a few casually placed objects, not a staged catalogue photograph or a CGI interior. \
No people, body parts, visible camera equipment, text, logos, watermarks, smoke or fog. \
Camera geometry, spacious composition and the requested lighting take priority over decorative location details.";
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
