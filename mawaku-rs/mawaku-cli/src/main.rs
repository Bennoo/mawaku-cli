use clap::Parser;
use mawaku_config::{Config, DEFAULT_PROMPT, load_or_init};
use mawaku_gemini::{PlaceDescription, craft_prompt, generate_image, generate_place_description};
use mawaku_image::{SaveImageOptions, save_base64_image};
use mawaku_utils::terminal::{ProgressStep, print_header, print_notice};
use mawaku_utils::{
    DEFAULT_FILE_NAME_PREFIX, ImageNameBuilder, ImageNameContext, distribute_prompt_details,
    format_context_line, list_or_unspecified, trimmed_or_none,
};
use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::time::Instant;

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
    /// Number of distinct image variants to generate (1-3).
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=3))]
    count: u8,
    /// Show full prompts and local reference details.
    #[arg(short, long)]
    verbose: bool,
    /// Optional season that informs the ambience of the scene.
    #[arg(long, value_name = "SEASON")]
    season: Option<String>,
    /// Optional time of day to tailor the lighting of the scene.
    #[arg(long = "time-of-day", value_name = "TIME")]
    time_of_day: Option<String>,
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
                "Complete place description:\nOptional local references: select only two or three compatible details, not the whole list. Keep them in the middle or far background. These references must not override camera geometry, room layout or the requested time of day.\nAmbiance: {}\nItems: {}\nKeywords: {}",
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
        "Scene timing:\n{}\n{}\nUse this timing for all light sources and the exterior view. Ignore conflicting lighting suggestions in the local references. If timing is unspecified, use soft overcast daytime light. If season is unspecified, keep seasonal decoration neutral.",
        format_context_line("Season", season),
        format_context_line("Time of day", time_of_day),
    );
    sections.push(timing_section);

    sections.join("\n\n")
}

/// Keep the shared photographic constraints while changing compatible scene details.
fn build_prompt_variants(
    instructions: &str,
    description: Option<&PlaceDescription>,
    season: Option<&str>,
    time_of_day: Option<&str>,
    count: u8,
) -> Vec<String> {
    if count == 1 {
        return vec![build_structured_prompt(
            instructions,
            description,
            season,
            time_of_day,
        )];
    }
    let arrangements = [
        "Arrange a reading chair beside a side window, with a low bookcase on the farther wall. Use a casually placed book as the small sign of daily life.",
        "Arrange a comfortable sofa along a side wall, with an open doorway establishing depth. Use a loosely folded throw as the small sign of daily life.",
        "Arrange a quiet study corner in the middle distance to one side, with a wooden chair and a small plant. Use a ceramic cup as the small sign of daily life.",
    ];
    (0..usize::from(count)).map(|index| {
        let selected = description.map(|details| PlaceDescription {
            ambiance: details.ambiance.clone(),
            items: distribute_prompt_details(&details.items, index, usize::from(count), 2),
            keywords: distribute_prompt_details(&details.keywords, index, usize::from(count), 1),
        });
        let mut prompt = build_structured_prompt(instructions, selected.as_ref(), season, time_of_day);
        prompt.push_str("\n\nScene arrangement:\n");
        prompt.push_str(arrangements[index]);
        prompt.push_str(" Keep these furnishings away from the camera and the caller's central area. Adapt local references to this arrangement only where compatible; preserve the camera geometry and requested timing.");
        prompt
    }).collect()
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

    let verbose = cli.verbose;
    let context = run(cli);
    let started = Instant::now();
    print_header(
        &context.location,
        context.season.as_deref(),
        context.time_of_day.as_deref(),
        context.count,
    );
    for message in &context.infos {
        print_notice(message, false);
    }
    for warning in &context.warnings {
        print_notice(warning, true);
    }

    let api_key = context
        .gemini_api_key
        .as_deref()
        .filter(|_| context.config_ready);
    let general_instructions = craft_prompt(DEFAULT_PROMPT, &context.location, None, None);
    let mut description = None;
    if let Some(api_key) = api_key {
        let progress = ProgressStep::new("Preparing local details");
        match generate_place_description(
            &context.location,
            context.season.as_deref().unwrap_or("any season"),
            api_key,
        ) {
            Ok(details) => {
                progress.finish(true, "ready");
                if verbose {
                    print_notice(&format!("Local references: {details}"), false);
                }
                description = Some(details);
            }
            Err(error) => {
                progress.finish(false, "using default scene details");
                print_notice(&error.to_string(), true);
            }
        }
    }
    let prompts = build_prompt_variants(
        &general_instructions,
        description.as_ref(),
        context.season.as_deref(),
        context.time_of_day.as_deref(),
        context.count,
    );
    let mut saved = Vec::new();
    for (index, prompt) in prompts.iter().enumerate() {
        // Preserve prompt output for pipes and prompt-only use, while keeping the terminal calm.
        if verbose || !io::stdout().is_terminal() || api_key.is_none() {
            if context.count > 1 {
                println!("=== Image variant {}/{} ===", index + 1, context.count);
            }
            println!("{prompt}");
        }
        let Some(api_key) = api_key else {
            continue;
        };
        let label = if context.count == 1 {
            "Background"
        } else {
            ["Reading corner", "Living room", "Study corner"][index]
        };
        let progress = ProgressStep::new(format!("{}/{}  {label}", index + 1, context.count));
        match generate_image(api_key, prompt) {
            Ok(response) => {
                let Some(image) = response.images.first() else {
                    progress.finish(false, "no image returned");
                    continue;
                };
                let stem = image_name_context.file_stem(index + 1);
                match save_base64_image(
                    &image.data,
                    SaveImageOptions {
                        file_stem: Some(&stem),
                        mime_type: image.mime_type.as_deref(),
                        output_dir: context.image_output_dir.as_deref(),
                    },
                ) {
                    Ok(path) => {
                        progress.finish(true, "saved");
                        saved.push(path);
                    }
                    Err(error) => {
                        progress.finish(false, "could not save image");
                        print_notice(&error.to_string(), true);
                    }
                }
            }
            Err(error) => {
                progress.finish(false, "generation failed");
                print_notice(&error.to_string(), true);
            }
        }
    }
    eprintln!();
    if api_key.is_some() {
        print_notice(
            &format!(
                "Saved {}/{} images in {:.1}s",
                saved.len(),
                context.count,
                started.elapsed().as_secs_f32()
            ),
            saved.len() != usize::from(context.count),
        );
        for path in saved {
            print_notice(&path.display().to_string(), false);
        }
    } else {
        print_notice(
            &format!("Prepared {} prompt(s) · no images generated", prompts.len()),
            false,
        );
    }
    eprintln!();
}

#[derive(Debug, Default)]
struct RunContext {
    count: u8,
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
        verbose: _,
        count,
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
                count,
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
                count,
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
