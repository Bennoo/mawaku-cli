use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_PROMPT: &str =
    "Imagine a workspace with immersive backgrounds! (Use --help for options.)";
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
    pub default_prompt: String,
    pub gemini_api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_prompt: DEFAULT_PROMPT.to_string(),
            gemini_api_key: DEFAULT_GEMINI_API_KEY.to_string(),
        }
    }
}

/// Loads the Mawaku configuration from disk, creating a default file if absent.
pub fn load_or_init() -> Result<LoadOutcome, ConfigError> {
    let path = config_file_path()?;

    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        let config = toml::from_str(&contents)?;
        Ok(LoadOutcome {
            config,
            path,
            created: false,
        })
    } else {
        ensure_parent_exists(&path)?;
        let config = Config::default();
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
    let base_dirs = BaseDirs::new().ok_or(ConfigError::ConfigDirUnavailable)?;
    Ok(base_dirs.home_dir().join(".mawaku").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_MUTEX: Mutex<()> = Mutex::new(());
    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn config_default_uses_default_prompt() {
        // Ensure the Config::default implementation returns the DEFAULT_PROMPT value.
        assert_eq!(Config::default().default_prompt, DEFAULT_PROMPT);
    }

    #[test]
    fn config_default_uses_empty_gemini_api_key() {
        assert!(Config::default().gemini_api_key.is_empty());
    }

    #[test]
    fn load_or_init_creates_file_with_empty_gemini_api_key() {
        with_isolated_home(|_| {
            let outcome = load_or_init().expect("load default config");
            assert!(outcome.created);
            assert_eq!(outcome.config.gemini_api_key, DEFAULT_GEMINI_API_KEY);

            let contents = fs::read_to_string(outcome.path).expect("read config");
            assert!(contents.contains("gemini_api_key = \"\""));
        });
    }

    fn with_isolated_home<F>(func: F)
    where
        F: FnOnce(&Path),
    {
        let _guard = TEST_MUTEX.lock().unwrap();
        let temp_home = create_unique_home();
        let snapshot = EnvSnapshot::capture();
        set_home_env(&temp_home);

        func(&temp_home);

        snapshot.restore();
        let _ = fs::remove_dir_all(&temp_home);
    }

    fn create_unique_home() -> PathBuf {
        let id = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "mawaku-config-test-home-{}-{}",
            std::process::id(),
            id
        ));
        fs::create_dir_all(&path).expect("create unique test home");
        path
    }

    fn set_home_env(path: &Path) {
        set_env("HOME", path.as_os_str());
        set_env("USERPROFILE", path.as_os_str());
    }

    struct EnvSnapshot {
        home: Option<OsString>,
        userprofile: Option<OsString>,
    }

    impl EnvSnapshot {
        fn capture() -> Self {
            Self {
                home: std::env::var_os("HOME"),
                userprofile: std::env::var_os("USERPROFILE"),
            }
        }

        fn restore(self) {
            if let Some(value) = self.home {
                set_env("HOME", &value);
            } else {
                remove_env("HOME");
            }

            if let Some(value) = self.userprofile {
                set_env("USERPROFILE", &value);
            } else {
                remove_env("USERPROFILE");
            }
        }
    }

    fn set_env(key: &str, value: &OsStr) {
        // SAFETY: `key` and `value` originate from ASCII string literals or formatter
        // output that never embed null bytes, satisfying the environment invariants.
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }
}
