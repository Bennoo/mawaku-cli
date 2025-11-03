use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// pub const DEFAULT_IMG_MODEL_VERSION: &str = "imagen-4.0-generate-001";
pub const DEFAULT_IMG_MODEL_VERSION: &str = "imagen-4.0-ultra-generate-001";
pub const DEFAULT_TEXT_MODEL_VERSION: &str = "gemini-2.5-flash";
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
    #[error("Failed to parse JSON response: {0}")]
    JsonParse(#[from] serde_json::Error),
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

// Text generation request structures matching Gemini API format
#[derive(Debug, Serialize)]
struct TextRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
    #[serde(rename = "responseSchema")]
    response_schema: ResponseSchema,
}

#[derive(Debug, Serialize)]
struct ResponseSchema {
    #[serde(rename = "type")]
    schema_type: String,
    properties: serde_json::Value,
    #[serde(rename = "propertyOrdering", skip_serializing_if = "Option::is_none")]
    property_ordering: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct Content<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Debug, Serialize)]
struct Part<'a> {
    text: &'a str,
}

// Text generation response structures
#[derive(Debug, Deserialize)]
pub struct GenerateContentResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
pub struct Candidate {
    pub content: ContentResponse,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContentResponse {
    pub parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PartResponse {
    pub text: String,
}

// Place description structured output
#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceDescription {
    pub ambiance: String,
    pub items: Vec<String>,
    pub keywords: Vec<String>,
}

impl std::fmt::Display for PlaceDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Ambiance: {}", self.ambiance)?;
        writeln!(f, "Items: {}", self.items.join(", "))?;
        write!(f, "Keywords: {}", self.keywords.join(", "))
    }
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

impl<'a> TextRequest<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part { text }],
            }],
            generation_config: None,
        }
    }

    fn with_schema(text: &'a str, generation_config: GenerationConfig) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part { text }],
            }],
            generation_config: Some(generation_config),
        }
    }
}

fn image_endpoint_url() -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_version}:predict",
        model_version = DEFAULT_IMG_MODEL_VERSION
    )
}

fn text_endpoint_url() -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_version}:generateContent",
        model_version = DEFAULT_TEXT_MODEL_VERSION
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
    let url = image_endpoint_url();
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

/// Submit a text generation request to Gemini's API.
///
/// # Errors
///
/// Returns [`GeminiError::MissingApiKey`] when the provided API key is empty or
/// whitespace only. Network and HTTP errors are surfaced via `reqwest`.
pub fn generate_text(api_key: &str, prompt: &str) -> Result<GenerateContentResponse, GeminiError> {
    if api_key.trim().is_empty() {
        return Err(GeminiError::MissingApiKey);
    }

    let client = Client::new();
    let url = text_endpoint_url();
    let request_body = TextRequest::new(prompt);

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()?;

    let response = response.error_for_status()?;
    let parsed = response.json::<GenerateContentResponse>()?;
    Ok(parsed)
}

pub fn generate_place_description(
    location: &str,
    api_key: &str,
) -> Result<PlaceDescription, GeminiError> {
    if api_key.trim().is_empty() {
        return Err(GeminiError::MissingApiKey);
    }

    let prompt = format!(
        "Describe the place called {location}. Provide a general ambiance description, \
         a list of potential items that might be found in a cozy interior view of this place, \
         and a list of keywords that capture the essence of this location."
    );

    // Build the schema for structured output
    let schema_properties = serde_json::json!({
        "ambiance": { "type": "STRING" },
        "items": {
            "type": "ARRAY",
            "items": { "type": "STRING" }
        },
        "keywords": {
            "type": "ARRAY",
            "items": { "type": "STRING" }
        }
    });

    let generation_config = GenerationConfig {
        response_mime_type: "application/json".to_string(),
        response_schema: ResponseSchema {
            schema_type: "OBJECT".to_string(),
            properties: schema_properties,
            property_ordering: Some(vec![
                "ambiance".to_string(),
                "items".to_string(),
                "keywords".to_string(),
            ]),
        },
    };

    let client = Client::new();
    let url = text_endpoint_url();
    let request_body = TextRequest::with_schema(&prompt, generation_config);

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()?;

    let response = response.error_for_status()?;
    let parsed = response.json::<GenerateContentResponse>()?;

    // Extract the JSON text from the first candidate's first part
    let json_text = parsed
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.as_str())
        .unwrap_or("{}");

    // Parse the JSON into PlaceDescription
    let place_description: PlaceDescription = serde_json::from_str(json_text)?;

    Ok(place_description)
}

#[cfg(test)]
mod tests;
