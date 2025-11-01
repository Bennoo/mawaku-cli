use super::*;
use mawaku_config::{DEFAULT_GEMINI_API_KEY, DEFAULT_PROMPT};
use mawaku_gemini::craft_prompt;
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
            location: "Hakone, Japan".to_string(),
            season: None,
            time_of_day: None,
            set_gemini_api_key: None,
        });

        let expected_prompt = craft_prompt(DEFAULT_PROMPT, "Hakone, Japan", None, None);
        assert_eq!(context.prompt, expected_prompt);
        assert!(context.config_ready);
        assert!(context.gemini_api_key.is_none());
        let expected_dir = home.join(".mawaku");
        assert_eq!(
            context.image_output_dir.as_deref(),
            Some(expected_dir.as_path())
        );
        assert!(
            context
                .warnings
                .iter()
                .any(|warning| warning.contains("GEMINI_API_KEY is not set"))
        );

        let config_path = expected_dir.join("config.toml");
        let contents = fs::read_to_string(config_path).expect("config written");
        assert!(contents.contains(&format!("gemini_api_key = \"{}\"", DEFAULT_GEMINI_API_KEY)));
        assert!(contents.contains(&format!(
            "image_output_dir = \"{}\"",
            expected_dir.to_string_lossy()
        )));
        assert!(
            !contents.contains("default_prompt"),
            "default_prompt should no longer be stored in the config file"
        );
    });
}

#[test]
fn run_updates_gemini_key_and_suppresses_warning() {
    with_isolated_home(|home| {
        let context = run(Cli {
            location: "Hakone, Japan".to_string(),
            season: None,
            time_of_day: None,
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

        assert!(context.config_ready);
        assert_eq!(context.gemini_api_key.as_deref(), Some("secret-key"));
        let expected_dir = home.join(".mawaku");
        assert_eq!(
            context.image_output_dir.as_deref(),
            Some(expected_dir.as_path())
        );

        let config_path = expected_dir.join("config.toml");
        let contents = fs::read_to_string(&config_path).expect("config written");
        assert!(contents.contains("gemini_api_key = \"secret-key\""));
        assert!(contents.contains(&format!(
            "image_output_dir = \"{}\"",
            expected_dir.to_string_lossy()
        )));
        assert!(
            !contents.contains("default_prompt"),
            "default_prompt should no longer be stored in the config file"
        );

        let second_run = run(Cli {
            location: "Hakone, Japan".to_string(),
            season: None,
            time_of_day: None,
            set_gemini_api_key: None,
        });

        assert!(
            !second_run
                .warnings
                .iter()
                .any(|warning| warning.contains("GEMINI_API_KEY is not set"))
        );
        assert!(second_run.config_ready);
        assert_eq!(second_run.gemini_api_key.as_deref(), Some("secret-key"));
        let expected_prompt = craft_prompt(DEFAULT_PROMPT, "Hakone, Japan", None, None);
        assert_eq!(second_run.prompt, expected_prompt);
        assert_eq!(
            second_run.image_output_dir.as_deref(),
            Some(expected_dir.as_path())
        );
    });
}

#[test]
fn image_name_context_builds_unique_file_stem() {
    let cli = Cli {
        location: "Hakone, Japan".to_string(),
        season: Some("Spring".to_string()),
        time_of_day: Some("Dusk".to_string()),
        set_gemini_api_key: None,
    };

    let context = ImageNameContext::new(&cli);
    let stem = context.file_stem(1);

    assert!(stem.starts_with("mawaku-hakone-jap-spring-dusk-p1-"));

    let (_, suffix) = stem
        .rsplit_once('-')
        .expect("file stem includes random suffix separator");
    assert_eq!(suffix.len(), RANDOM_SUFFIX_LENGTH);

    let mut chars: Vec<char> = suffix.chars().collect();
    chars.sort_unstable();
    chars.dedup();
    assert_eq!(chars.len(), RANDOM_SUFFIX_LENGTH);
}

#[test]
fn image_name_context_truncates_long_components() {
    let cli = Cli {
        location: "Extremely Long Location Name That Keeps Going".to_string(),
        season: Some("Supercalifragilisticexpialidocious".to_string()),
        time_of_day: Some("Midnight Sun Time".to_string()),
        set_gemini_api_key: None,
    };

    let context = ImageNameContext::new(&cli);
    let stem = context.file_stem(2);
    let pattern = format!("-p{}-", 2);
    let (base, _) = stem
        .split_once(&pattern)
        .expect("file stem includes prediction index separator");

    assert!(stem.starts_with("mawaku-extremely-supercalif-midnight-s-p2-"));
    assert_eq!(base, "mawaku-extremely-supercalif-midnight-s");

    let location_component =
        component_token(&cli.location).expect("location component slug exists");
    assert_eq!(location_component, "extremely");

    let season_component = component_token(cli.season.as_deref().unwrap())
        .expect("season component slug exists");
    assert_eq!(season_component.len(), PARAM_COMPONENT_MAX_LEN);
    assert_eq!(season_component, "supercalif");

    let time_component = component_token(cli.time_of_day.as_deref().unwrap())
        .expect("time component slug exists");
    assert_eq!(time_component.len(), PARAM_COMPONENT_MAX_LEN);
    assert_eq!(time_component, "midnight-s");
}
