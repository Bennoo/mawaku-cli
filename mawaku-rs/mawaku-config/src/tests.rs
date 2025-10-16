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
