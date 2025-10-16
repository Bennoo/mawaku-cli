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
mod tests;
