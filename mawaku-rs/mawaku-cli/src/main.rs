use clap::Parser;
use mawaku_config::{Config, DEFAULT_PROMPT, load_or_init};
use mawaku_gemini::{
    GeminiError, PlaceDescription, PredictResponse, craft_prompt, generate_image,
    generate_place_description,
};
use mawaku_image::{SaveImageOptions, save_base64_image};
use mawaku_utils::{
    DEFAULT_FILE_NAME_PREFIX, ImageNameBuilder, ImageNameContext, format_context_line,
    list_or_unspecified, trimmed_or_none,
};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

const GEMINI_KEY_WARNING_PREFIX: &str =
    "Warning: Gemini API key environment variable is missing. Export it before running Mawaku: ";

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

fn build_structured_prompt(
    general_instructions: &str,
    description: Option<&PlaceDescription>,
    season: Option<&str>,
    time_of_day: Option<&str>,
) -> String {
    let mut sections = Vec::new();

    let instructions = general_instructions.trim();
    if !instructions.is_empty() {
        sections.push(instructions.to_string());
    }

    let place_section = match description {
        Some(details) => {
            let ambiance =
                trimmed_or_none(Some(details.ambiance.as_str())).unwrap_or("Unspecified");
            let items = list_or_unspecified(&details.items);
            let keywords = list_or_unspecified(&details.keywords);
            format!(
                "Complete place description:\nUse one or many of these details:\nAmbiance: {}\nItems: {}\nKeywords: {}",
                ambiance, items, keywords
            )
        }
        None => {
            "Complete place description:\nUse one or many of these details:\nAmbiance: Unspecified\nItems: Unspecified\nKeywords: Unspecified"
                .to_string()
        }
    };
    sections.push(place_section);

    let timing_section = format!(
        "Scene timing:\n{}\n{}",
        format_context_line("Season", season),
        format_context_line("Time of day", time_of_day),
    );
    sections.push(timing_section);

    sections.join("\n\n")
}

fn build_image_name_context(cli: &Cli) -> ImageNameContext {
    let mut builder = ImageNameBuilder::new(DEFAULT_FILE_NAME_PREFIX);
    builder.push_component(Some(cli.location.as_str()));
    builder.push_component(cli.season.as_deref());
    builder.push_component(cli.time_of_day.as_deref());
    builder.build()
}

fn main() {
    let cli = Cli::parse();
    let image_name_context = build_image_name_context(&cli);

    let context = run(cli);

    for message in &context.infos {
        eprintln!("{message}");
    }

    for warning in &context.warnings {
        eprintln!("{warning}");
    }

    let general_instructions = craft_prompt(DEFAULT_PROMPT, &context.location, None, None);
    let mut prompt = build_structured_prompt(
        general_instructions.as_str(),
        None,
        context.season.as_deref(),
        context.time_of_day.as_deref(),
    );

    if context.config_ready
        && let Some(api_key) = context.gemini_api_key.as_deref()
    {
        let season = context.season.as_deref().unwrap_or("any season");
        match generate_place_description(&context.location, season, api_key) {
            Ok(description) => {
                eprintln!("Gemini place description: {}", description);
                prompt = build_structured_prompt(
                    general_instructions.as_str(),
                    Some(&description),
                    context.season.as_deref(),
                    context.time_of_day.as_deref(),
                );
            }
            Err(error) => {
                eprintln!("Warning: failed to generate place description via Gemini ({error}).");
            }
        }
        match generate_image_with_progress(api_key, &prompt) {
            Some(Ok(response)) => {
                eprintln!(
                    "Gemini generated {} prediction(s).",
                    response.predictions.len()
                );

                for (index, prediction) in response.predictions.iter().enumerate() {
                    let display_index = index + 1;
                    match prediction.bytes_base64_encoded.as_deref() {
                        Some(encoded) => {
                            let file_stem = image_name_context.file_stem(display_index);
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

    println!("{prompt}");
}

#[derive(Debug, Default)]
struct RunContext {
    #[cfg_attr(not(test), allow(dead_code))]
    prompt: String,
    location: String,
    infos: Vec<String>,
    warnings: Vec<String>,
    gemini_api_key: Option<String>,
    config_ready: bool,
    image_output_dir: Option<PathBuf>,
    season: Option<String>,
    time_of_day: Option<String>,
}

fn run(cli: Cli) -> RunContext {
    let Cli {
        location,
        season,
        time_of_day,
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

            let config = outcome.config;

            let (gemini_api_key, warning) = resolve_gemini_api_key(&config);
            if let Some(message) = warning {
                warnings.push(message);
            }

            let prompt_value = craft_prompt(
                DEFAULT_PROMPT,
                &location,
                season.as_deref(),
                time_of_day.as_deref(),
            );
            let gemini_api_key = gemini_api_key.clone();
            let image_output_dir = Some(PathBuf::from(&config.image_output_dir));

            RunContext {
                prompt: prompt_value,
                location: location.to_string(),
                infos,
                warnings,
                gemini_api_key,
                config_ready: true,
                image_output_dir,
                season: season.clone(),
                time_of_day: time_of_day.clone(),
            }
        }
        Err(error) => {
            warnings.push(format!(
                "Warning: failed to load Mawaku configuration ({error}). Falling back to defaults."
            ));

            let config = Config::default();

            let (gemini_api_key, warning) = resolve_gemini_api_key(&config);
            if let Some(message) = warning {
                warnings.push(message);
            }

            let prompt_value = craft_prompt(
                DEFAULT_PROMPT,
                &location,
                season.as_deref(),
                time_of_day.as_deref(),
            );
            let gemini_api_key = gemini_api_key.clone();
            let image_output_dir = Some(PathBuf::from(&config.image_output_dir));

            RunContext {
                prompt: prompt_value,
                location: location.to_string(),
                infos,
                warnings,
                gemini_api_key,
                config_ready: false,
                image_output_dir,
                season: season.clone(),
                time_of_day: time_of_day.clone(),
            }
        }
    }
}

fn resolve_gemini_api_key(config: &Config) -> (Option<String>, Option<String>) {
    let env_var = config.gemini_api.api_key_env_var();
    match env::var(env_var) {
        Ok(value) if !value.trim().is_empty() => (Some(value), None),
        _ => (None, Some(format!("{GEMINI_KEY_WARNING_PREFIX}{env_var}."))),
    }
}

#[cfg(test)]
mod tests;
