use clap::Parser;
use mawaku_config::{Config, DEFAULT_PROMPT, load_or_init};

/// Mawaku CLI entry point.
///
/// Mawaku will translate natural language scene descriptions into
/// prompts for background generators such as Google Imagen or OpenAI's DALL-E.
#[derive(Parser, Debug)]
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
}

fn main() {
    let cli = Cli::parse();

    let fallback_prompt = match load_or_init() {
        Ok(outcome) => {
            if outcome.created {
                eprintln!(
                    "Created Mawaku configuration at {} with the default prompt: \"{DEFAULT_PROMPT}\"",
                    outcome.path.display()
                );
            }
            outcome.config.default_prompt
        }
        Err(error) => {
            eprintln!(
                "Warning: failed to load Mawaku configuration ({error}). Falling back to defaults."
            );
            Config::default().default_prompt
        }
    };

    let prompt = cli.prompt.unwrap_or(fallback_prompt);

    println!("{prompt}");
}
