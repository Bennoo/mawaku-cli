use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_MODEL_VERSION: &str = "imagen-4.0-generate-001";
pub const DEFAULT_SAMPLE_COUNT: u32 = 2;
pub const DEFAULT_ASPECT_RATIO: &str = "16:9";

fn normalized(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Build a descriptive prompt for Gemini based on contextual inputs.
///
/// The `base_prompt` establishes the overall art direction, while the
/// location, season, and time-of-day arguments provide scene-specific
/// details. Empty strings are ignored so callers can pass user-provided
/// values without additional validation.
pub fn craft_prompt(
    base_prompt: &str,
    location: &str,
    season: Option<&str>,
    time_of_day: Option<&str>,
) -> String {
    let mut segments: Vec<String> = Vec::new();

    if let Some(base) = normalized(base_prompt) {
        segments.push(base.to_string());
    }

    if let Some(loc) = normalized(location) {
        segments.push(format!(
            "Set the scene in {loc} and showcase the atmosphere from a cosy, lived-in interior perspective."
        ));
    }

    if let Some(season_value) = season.and_then(normalized) {
        segments.push(format!("It is {season_value}."));
    }

    if let Some(time_value) = time_of_day.and_then(normalized) {
        segments.push(format!("Capture the lighting of {time_value}."));
    }

    if segments.is_empty() {
        String::new()
    } else {
        segments.join(" ")
    }
}

#[derive(Debug, Error)]
pub enum GeminiError {
    #[error("Gemini API key is missing")]
    MissingApiKey,
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Deserialize)]
pub struct PredictResponse {
    #[serde(default)]
    pub predictions: Vec<PredictPrediction>,
}

#[derive(Debug, Deserialize)]
pub struct PredictPrediction {
    #[serde(rename = "bytesBase64Encoded")]
    pub bytes_base64_encoded: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct PredictRequest<'a> {
    instances: Vec<Instance<'a>>,
    parameters: Parameters,
}

#[derive(Debug, Serialize)]
struct Instance<'a> {
    prompt: &'a str,
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(rename = "sampleCount")]
    sample_count: u32,
    #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
}

impl<'a> PredictRequest<'a> {
    fn new(prompt: &'a str, sample_count: u32, aspect_ratio: Option<String>) -> Self {
        Self {
            instances: vec![Instance { prompt }],
            parameters: Parameters {
                sample_count,
                aspect_ratio,
            },
        }
    }
}

fn endpoint_url() -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_version}:predict",
        model_version = DEFAULT_MODEL_VERSION
    )
}

/// Submit an image generation request to Gemini's Imagen 4 API.
///
/// The request targets Gemini's hosted Imagen 4 endpoint. Future iterations can
/// expose configuration hooks for model selection and regional routing.
///
/// # Errors
///
/// Returns [`GeminiError::MissingApiKey`] when the provided API key is empty or
/// whitespace only. Network and HTTP errors are surfaced via `reqwest`.
pub fn generate_image(api_key: &str, prompt: &str) -> Result<PredictResponse, GeminiError> {
    if api_key.trim().is_empty() {
        return Err(GeminiError::MissingApiKey);
    }

    let client = Client::new();
    let url = endpoint_url();
    let request_body = PredictRequest::new(
        prompt,
        DEFAULT_SAMPLE_COUNT,
        Some(DEFAULT_ASPECT_RATIO.to_string()),
    );

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&request_body)
        .send()?;

    let response = response.error_for_status()?;
    let parsed = response.json::<PredictResponse>()?;
    Ok(parsed)
}

#[cfg(test)]
mod tests;
