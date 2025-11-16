use super::*;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use toml::Value;

static TEST_MUTEX: Mutex<()> = Mutex::new(());
static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn config_default_sets_gemini_api_env_var() {
    let config = Config::default();
    assert_eq!(
        config.gemini_api.api_key_env_var,
        DEFAULT_GEMINI_API_KEY_ENV_VAR
    );
}

#[test]
fn config_default_sets_image_output_dir() {
    let config = Config::default();
    assert!(!config.image_output_dir.trim().is_empty());
}

#[test]
fn load_or_init_creates_file_with_default_gemini_api_env_var() {
    with_isolated_home(|_| {
        let outcome = load_or_init().expect("load default config");
        assert!(outcome.created);
        assert_eq!(
            outcome.config.gemini_api.api_key_env_var,
            DEFAULT_GEMINI_API_KEY_ENV_VAR
        );
        assert!(!outcome.config.image_output_dir.trim().is_empty());

        let contents = fs::read_to_string(outcome.path).expect("read config");
        let parsed: Value = contents.parse().expect("config is valid TOML");
        let gemini = parsed
            .get("gemini_api")
            .and_then(Value::as_table)
            .expect("gemini table exists");
        assert_eq!(
            gemini.get("api_key_env_var").and_then(Value::as_str),
            Some(DEFAULT_GEMINI_API_KEY_ENV_VAR)
        );
        assert!(contents.contains("image_output_dir ="));
        assert!(!contents.contains("default_prompt"));
    });
}

#[test]
fn load_or_init_backfills_missing_image_output_dir() {
    with_isolated_home(|home| {
        let config_dir = home.join(".mawaku");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let path = config_dir.join("config.toml");
        fs::write(
            &path,
            r#"
default_prompt = "Test"
gemini_api_key = "super-secret"
"#,
        )
        .expect("write legacy config");

        let outcome = load_or_init().expect("load legacy config");
        assert!(!outcome.created);
        let expected_dir = config_dir.to_string_lossy().into_owned();
        assert_eq!(outcome.config.image_output_dir, expected_dir);

        let contents = fs::read_to_string(&path).expect("read config");
        assert!(contents.contains(&format!("image_output_dir = \"{expected_dir}\"")));
        assert!(!contents.contains("default_prompt"));
        assert!(!contents.contains("gemini_api_key"));
        assert!(contents.contains("[gemini_api]"));
    });
}

#[test]
fn load_or_init_rewrites_legacy_environment_mapping() {
    with_isolated_home(|home| {
        let config_dir = home.join(".mawaku");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let path = config_dir.join("config.toml");
        fs::write(
            &path,
            r#"
[gemini_api]
environment = "staging"
[gemini_api.environments]
staging = "CUSTOM_GEMINI"
"#,
        )
        .expect("write legacy config");

        let outcome = load_or_init().expect("load rewritten config");
        assert!(!outcome.created);
        assert_eq!(outcome.config.gemini_api.api_key_env_var, "CUSTOM_GEMINI");

        let contents = fs::read_to_string(&path).expect("read config");
        assert!(contents.contains("api_key_env_var = \"CUSTOM_GEMINI\""));
        assert!(!contents.contains("environment"));
        assert!(!contents.contains("environments"));
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
    remove_env(DEFAULT_GEMINI_API_KEY_ENV_VAR);

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
    gemini_api_key: Option<OsString>,
}

impl EnvSnapshot {
    fn capture() -> Self {
        Self {
            home: std::env::var_os("HOME"),
            userprofile: std::env::var_os("USERPROFILE"),
            gemini_api_key: std::env::var_os(DEFAULT_GEMINI_API_KEY_ENV_VAR),
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

        if let Some(value) = self.gemini_api_key {
            set_env(DEFAULT_GEMINI_API_KEY_ENV_VAR, &value);
        } else {
            remove_env(DEFAULT_GEMINI_API_KEY_ENV_VAR);
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
