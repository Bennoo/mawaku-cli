use clap::Parser;
use mawaku_config::{Config, DEFAULT_PROMPT, load_or_init, save};
use mawaku_gemini::{GeminiError, PredictResponse, craft_prompt, generate_image};
use mawaku_image::{SaveImageOptions, save_base64_image};
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

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
    /// Location that should anchor the generated background.
    #[arg(long, value_name = "LOCATION")]
    location: String,
    /// Optional season that informs the ambience of the scene.
    #[arg(long, value_name = "SEASON")]
    season: Option<String>,
    /// Optional time of day to tailor the lighting of the scene.
    #[arg(long = "time-of-day", value_name = "TIME")]
    time_of_day: Option<String>,
    /// Set the Gemini API key persisted in the Mawaku config file.
    #[arg(long, value_name = "KEY")]
    set_gemini_api_key: Option<String>,
}

fn generate_image_with_progress(
    api_key: &str,
    prompt: &str,
) -> Option<Result<PredictResponse, GeminiError>> {
    let api_key = api_key.to_string();
    let prompt = prompt.to_string();

    let handle = thread::Builder::new()
        .name("gemini-image-request".into())
        .spawn(move || generate_image(&api_key, &prompt))
        .expect("spawn gemini image request");

    const SPINNER_FRAMES: &[&str] = &["|", "/", "-", "\\"];
    let mut frame_index = 0;
    let interval = Duration::from_millis(200);
    let start = Instant::now();

    eprint!("Generating image ");
    let _ = io::stderr().flush();

    while !handle.is_finished() {
        eprint!("\rGenerating image {}", SPINNER_FRAMES[frame_index]);
        let _ = io::stderr().flush();
        frame_index = (frame_index + 1) % SPINNER_FRAMES.len();
        thread::sleep(interval);
    }

    match handle.join() {
        Ok(result) => {
            eprintln!(
                "\rGenerating image ... finished in {:.1}s",
                start.elapsed().as_secs_f32()
            );
            Some(result)
        }
        Err(_) => {
            eprintln!("\rGenerating image ... failed: worker panicked");
            None
        }
    }
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

    if context.config_ready {
        if let Some(api_key) = context.gemini_api_key.as_deref() {
            match generate_image_with_progress(api_key, &context.prompt) {
                Some(Ok(response)) => {
                    eprintln!(
                        "Gemini generated {} prediction(s).",
                        response.predictions.len()
                    );

                    for (index, prediction) in response.predictions.iter().enumerate() {
                        let display_index = index + 1;
                        match prediction.bytes_base64_encoded.as_deref() {
                            Some(encoded) => {
                                let file_stem = format!("mawaku-generated-{display_index}");
                                let output_dir = context.image_output_dir.as_deref();
                                let options = SaveImageOptions {
                                    file_stem: Some(file_stem.as_str()),
                                    mime_type: prediction.mime_type.as_deref(),
                                    output_dir,
                                };

                                match save_base64_image(encoded, options) {
                                    Ok(path) => {
                                        eprintln!(
                                            "Saved prediction #{display_index} to {}",
                                            path.display()
                                        );
                                    }
                                    Err(error) => {
                                        eprintln!(
                                            "Warning: failed to save prediction #{display_index} ({error})."
                                        );
                                    }
                                }
                            }
                            None => {
                                eprintln!(
                                    "Warning: prediction #{display_index} did not include encoded image bytes."
                                );
                            }
                        }
                    }
                }
                Some(Err(error)) => {
                    eprintln!("Warning: failed to generate image via Gemini ({error}).");
                }
                None => {
                    eprintln!("Warning: image generation request ended unexpectedly.");
                }
            }
        }
    }

    println!("{}", context.prompt);
}

#[derive(Debug, Default)]
struct RunContext {
    prompt: String,
    infos: Vec<String>,
    warnings: Vec<String>,
    gemini_api_key: Option<String>,
    config_ready: bool,
    image_output_dir: Option<PathBuf>,
}

fn run(cli: Cli) -> RunContext {
    let Cli {
        location,
        season,
        time_of_day,
        set_gemini_api_key,
    } = cli;

    let mut infos = Vec::new();
    let mut warnings = Vec::new();

    match load_or_init() {
        Ok(outcome) => {
            if outcome.created {
                infos.push(format!(
                    "Created Mawaku configuration at {}",
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

            let has_api_key = !config.gemini_api_key.trim().is_empty();
            if !has_api_key {
                warnings.push(GEMINI_KEY_WARNING.to_string());
            }

            let prompt_value = craft_prompt(
                DEFAULT_PROMPT,
                &location,
                season.as_deref(),
                time_of_day.as_deref(),
            );
            let gemini_api_key = has_api_key.then(|| config.gemini_api_key.clone());
            let image_output_dir = Some(PathBuf::from(&config.image_output_dir));

            RunContext {
                prompt: prompt_value,
                infos,
                warnings,
                gemini_api_key,
                config_ready: true,
                image_output_dir,
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

            let has_api_key = !config.gemini_api_key.trim().is_empty();
            if !has_api_key {
                warnings.push(GEMINI_KEY_WARNING.to_string());
            }

            let prompt_value = craft_prompt(
                DEFAULT_PROMPT,
                &location,
                season.as_deref(),
                time_of_day.as_deref(),
            );
            let gemini_api_key = has_api_key.then(|| config.gemini_api_key.clone());
            let image_output_dir = Some(PathBuf::from(&config.image_output_dir));

            RunContext {
                prompt: prompt_value,
                infos,
                warnings,
                gemini_api_key,
                config_ready: false,
                image_output_dir,
            }
        }
    }
}

#[cfg(test)]
mod tests;
