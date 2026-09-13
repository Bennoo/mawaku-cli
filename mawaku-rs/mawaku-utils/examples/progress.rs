//! Preview terminal presentation without API requests or generated images.
use mawaku_utils::terminal::{ProgressStep, print_header, print_notice};
use std::{thread, time::Duration};

fn main() {
    print_header("Tokyo, Asakusa", Some("Autumn"), Some("Morning"), 3);
    print_notice("UI preview — no images will be generated", false);
    for (label, success, detail) in [
        ("Preparing local details", true, "ready"),
        ("1/3  Reading corner", true, "saved (demo)"),
        ("2/3  Living room", false, "generation failed (demo)"),
        ("3/3  Study corner", true, "saved (demo)"),
    ] {
        let progress = ProgressStep::new(label);
        thread::sleep(Duration::from_millis(400));
        progress.finish(success, detail);
    }
    print_notice("Preview complete", false);
}
