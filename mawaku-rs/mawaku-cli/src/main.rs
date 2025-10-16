use clap::Parser;
use mawaku_config::{Config, DEFAULT_PROMPT, load_or_init, save};

const GEMINI_KEY_WARNING: &str =
    "Warning: GEMINI_API_KEY is not set. Use `mawaku --set-gemini-api-key <KEY>` to configure it.";

/// Mawaku CLI entry point.
///
/// Mawaku will translate natural language scene descriptions into
/// prompts for background generators such as Google Imagen or OpenAI's DALL-E.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "mawaku",
    author,
    version,
    about = "Generate video-call backgrounds by describing a place.",
    long_about = None
)]
struct Cli {
    /// Describe the workspace background you want to generate.
    #[arg(long, value_name = "TEXT")]
    prompt: Option<String>,
    /// Set the Gemini API key persisted in the Mawaku config file.
    #[arg(long, value_name = "KEY")]
    set_gemini_api_key: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let context = run(cli);

    for message in &context.infos {
        eprintln!("{message}");
    }

    for warning in &context.warnings {
        eprintln!("{warning}");
    }

    println!("{}", context.prompt);
}

#[derive(Debug, Default)]
struct RunContext {
    prompt: String,
    infos: Vec<String>,
    warnings: Vec<String>,
}

fn run(cli: Cli) -> RunContext {
    let Cli {
        prompt,
        set_gemini_api_key,
    } = cli;

    let mut infos = Vec::new();
    let mut warnings = Vec::new();

    match load_or_init() {
        Ok(outcome) => {
            if outcome.created {
                infos.push(format!(
                    "Created Mawaku configuration at {} with the default prompt: \"{DEFAULT_PROMPT}\"",
                    outcome.path.display()
                ));
            }

            let mut config = outcome.config;

            if let Some(key) = set_gemini_api_key.clone() {
                config.gemini_api_key = key;
                match save(&config, &outcome.path) {
                    Ok(()) => infos.push(format!(
                        "Updated GEMINI_API_KEY in {}",
                        outcome.path.display()
                    )),
                    Err(error) => warnings.push(format!(
                        "Warning: failed to update GEMINI_API_KEY ({error})."
                    )),
                }
            }

            if config.gemini_api_key.trim().is_empty() {
                warnings.push(GEMINI_KEY_WARNING.to_string());
            }

            let prompt_value = prompt.unwrap_or_else(|| config.default_prompt.clone());

            RunContext {
                prompt: prompt_value,
                infos,
                warnings,
            }
        }
        Err(error) => {
            warnings.push(format!(
                "Warning: failed to load Mawaku configuration ({error}). Falling back to defaults."
            ));

            if set_gemini_api_key.is_some() {
                warnings.push(
                    "Warning: cannot update GEMINI_API_KEY because the configuration could not be loaded."
                        .to_string(),
                );
            }

            let config = Config::default();

            if config.gemini_api_key.trim().is_empty() {
                warnings.push(GEMINI_KEY_WARNING.to_string());
            }

            let prompt_value = prompt.unwrap_or_else(|| config.default_prompt.clone());

            RunContext {
                prompt: prompt_value,
                infos,
                warnings,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mawaku_config::DEFAULT_GEMINI_API_KEY;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_MUTEX: Mutex<()> = Mutex::new(());
    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
            "mawaku-cli-test-home-{}-{}",
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
        // SAFETY: keys and values stem from ASCII literals or formatted identifiers
        // without interior null bytes, maintaining environment invariants.
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn run_warns_when_gemini_key_missing() {
        with_isolated_home(|home| {
            let context = run(Cli {
                prompt: None,
                set_gemini_api_key: None,
            });

            assert_eq!(context.prompt, DEFAULT_PROMPT);
            assert!(
                context
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("GEMINI_API_KEY is not set"))
            );

            let config_path = home.join(".mawaku").join("config.toml");
            let contents = fs::read_to_string(config_path).expect("config written");
            assert!(contents.contains(&format!("gemini_api_key = \"{}\"", DEFAULT_GEMINI_API_KEY)));
        });
    }

    #[test]
    fn run_updates_gemini_key_and_suppresses_warning() {
        with_isolated_home(|home| {
            let context = run(Cli {
                prompt: None,
                set_gemini_api_key: Some("secret-key".to_string()),
            });

            assert!(
                context
                    .infos
                    .iter()
                    .any(|info| info.contains("Updated GEMINI_API_KEY"))
            );
            assert!(
                !context
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("GEMINI_API_KEY is not set"))
            );

            let config_path = home.join(".mawaku").join("config.toml");
            let contents = fs::read_to_string(&config_path).expect("config written");
            assert!(contents.contains("gemini_api_key = \"secret-key\""));

            let second_run = run(Cli {
                prompt: None,
                set_gemini_api_key: None,
            });

            assert!(
                !second_run
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("GEMINI_API_KEY is not set"))
            );
            assert_eq!(second_run.prompt, DEFAULT_PROMPT);
        });
    }
}
