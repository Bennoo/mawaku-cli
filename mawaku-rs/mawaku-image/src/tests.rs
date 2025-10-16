use super::*;
use std::fs;
use std::path::PathBuf;

fn unique_temp_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let id = format!(
        "mawaku-image-test-{}-{}",
        std::process::id(),
        timestamp_suffix()
    );
    let dir = base.join(id);
    fs::create_dir_all(&dir).expect("create temp directory");
    dir
}

#[test]
fn saves_image_to_specified_directory() {
    let dir = unique_temp_dir();
    let options = SaveImageOptions {
        file_stem: Some("custom-name"),
        mime_type: Some("image/png"),
        output_dir: Some(dir.as_path()),
    };

    let path = save_base64_image("aGVsbG8=", options).expect("save image succeeds");
    assert_eq!(path, dir.join("custom-name.png"));

    let bytes = fs::read(&path).expect("read saved image");
    assert_eq!(bytes, b"hello");

    fs::remove_file(&path).ok();
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn default_output_directory_uses_application_path() {
    let options = SaveImageOptions {
        file_stem: Some("mawaku-test-default"),
        mime_type: Some("image/png"),
        output_dir: None,
    };

    let path = save_base64_image("aGVsbG8=", options).expect("save image with default directory");
    let exe_dir = std::env::current_exe()
        .expect("determine current exe")
        .parent()
        .expect("exe has parent")
        .to_path_buf();
    assert_eq!(path.parent(), Some(exe_dir.as_path()));

    fs::remove_file(&path).ok();
}

#[test]
fn empty_payload_is_rejected() {
    let error = save_base64_image("", SaveImageOptions::default()).expect_err("empty payload");
    assert!(matches!(error, ImageSaveError::EmptyPayload));
}
