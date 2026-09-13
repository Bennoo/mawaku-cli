//! Terminal presentation shared by CLI workflows. All UI output goes to stderr.
use console::Style;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

fn styled(text: &str, color: &str) -> String {
    let style = match color {
        "green" => Style::new().green(),
        "yellow" => Style::new().yellow(),
        "cyan" => Style::new().cyan().bold(),
        _ => Style::new().dim(),
    };
    style.for_stderr().apply_to(text).to_string()
}

pub fn print_header(location: &str, season: Option<&str>, time: Option<&str>, count: u8) {
    eprintln!("\n  {}", styled("Mawaku", "cyan"));
    let scene = [Some(location), season, time]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    eprintln!("  {}", styled(&scene, "dim"));
    eprintln!(
        "  {}\n",
        styled(&format!("{count} image variant(s)"), "dim")
    );
}

pub fn print_notice(message: &str, warning: bool) {
    let marker = if warning { "!" } else { "·" };
    eprintln!(
        "  {} {message}",
        styled(marker, if warning { "yellow" } else { "dim" })
    );
}

/// An indeterminate operation: animate on terminals, emit plain lines in logs.
/// Steady ticks run independently while the calling thread performs blocking I/O.
pub struct ProgressStep {
    bar: ProgressBar,
    label: String,
    started: Instant,
}

impl ProgressStep {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        let animated = io::stderr().is_terminal()
            && std::env::var_os("TERM").as_deref() != Some(std::ffi::OsStr::new("dumb"));
        let bar = ProgressBar::with_draw_target(
            None,
            if animated {
                ProgressDrawTarget::stderr()
            } else {
                ProgressDrawTarget::hidden()
            },
        );
        let template = if std::env::var_os("NO_COLOR").is_some() {
            "  {spinner} {msg}  {elapsed_precise}"
        } else {
            "  {spinner:.cyan} {msg}  {elapsed_precise:.dim}"
        };
        bar.set_style(
            ProgressStyle::with_template(template)
                .expect("valid progress template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        bar.set_message(label.clone());
        if animated {
            bar.enable_steady_tick(Duration::from_millis(80));
        } else {
            eprintln!("  > {label}");
        }
        Self {
            bar,
            label,
            started: Instant::now(),
        }
    }

    pub fn finish(self, success: bool, detail: &str) {
        self.bar.finish_and_clear();
        let marker = if success { "✓" } else { "!" };
        eprintln!(
            "  {} {} — {} {}",
            styled(marker, if success { "green" } else { "yellow" }),
            self.label,
            detail,
            styled(
                &format!("({:.1}s)", self.started.elapsed().as_secs_f32()),
                "dim"
            )
        );
    }
}

impl Drop for ProgressStep {
    fn drop(&mut self) {
        self.bar.finish_and_clear();
    }
}
