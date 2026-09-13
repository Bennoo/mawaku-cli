use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_IMG_MODEL_VERSION: &str = "gemini-3-pro-image";
pub const DEFAULT_TEXT_MODEL_VERSION: &str = "gemini-3.6-flash";
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

#[derive(Debug)]
pub struct ImageGenerationResponse {
    pub images: Vec<GeneratedImage>,
}

#[derive(Debug)]
pub struct GeneratedImage {
    pub data: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImageRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: ImageGenerationConfig<'a>,
}

#[derive(Debug, Serialize)]
struct ImageGenerationConfig<'a> {
    #[serde(rename = "responseModalities")]
    response_modalities: Vec<&'a str>,
    #[serde(rename = "imageConfig")]
    image_config: ImageConfig<'a>,
}

#[derive(Debug, Serialize)]
struct ImageConfig<'a> {
    #[serde(rename = "aspectRatio")]
    aspect_ratio: &'a str,
}

#[derive(Debug, Deserialize)]
struct ImageGenerateContentResponse {
    #[serde(default)]
    candidates: Vec<ImageCandidate>,
}

#[derive(Debug, Deserialize)]
struct ImageCandidate {
    content: ImageContentResponse,
}

#[derive(Debug, Deserialize)]
struct ImageContentResponse {
    #[serde(default)]
    parts: Vec<ImagePartResponse>,
}

#[derive(Debug, Deserialize)]
struct ImagePartResponse {
    #[serde(rename = "inlineData")]
    inline_data: Option<InlineData>,
}

#[derive(Debug, Deserialize)]
struct InlineData {
    data: String,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
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

impl<'a> ImageRequest<'a> {
    fn new(prompt: &'a str, aspect_ratio: &'a str) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
            generation_config: ImageGenerationConfig {
                response_modalities: vec!["IMAGE"],
                image_config: ImageConfig { aspect_ratio },
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
        "https://generativelanguage.googleapis.com/v1beta/models/{model_version}:generateContent",
        model_version = DEFAULT_IMG_MODEL_VERSION
    )
}

fn text_endpoint_url() -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_version}:generateContent",
        model_version = DEFAULT_TEXT_MODEL_VERSION
    )
}

/// Submit an image generation request to Gemini's native image-generation API.
///
/// # Errors
///
/// Returns [`GeminiError::MissingApiKey`] when the provided API key is empty or
/// whitespace only. Network and HTTP errors are surfaced via `reqwest`.
pub fn generate_image(api_key: &str, prompt: &str) -> Result<ImageGenerationResponse, GeminiError> {
    if api_key.trim().is_empty() {
        return Err(GeminiError::MissingApiKey);
    }

    let client = Client::new();
    let url = image_endpoint_url();
    let request_body = ImageRequest::new(prompt, DEFAULT_ASPECT_RATIO);

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&request_body)
        .send()?;

    let response = response.error_for_status()?;
    let parsed = response.json::<ImageGenerateContentResponse>()?;
    Ok(image_generation_response(parsed))
}

fn image_generation_response(response: ImageGenerateContentResponse) -> ImageGenerationResponse {
    let images = response
        .candidates
        .into_iter()
        .flat_map(|candidate| candidate.content.parts)
        .filter_map(|part| part.inline_data)
        .map(|inline_data| GeneratedImage {
            data: inline_data.data,
            mime_type: inline_data.mime_type,
        })
        .collect();
    ImageGenerationResponse { images }
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
    season: &str,
    api_key: &str,
) -> Result<PlaceDescription, GeminiError> {
    if api_key.trim().is_empty() {
        return Err(GeminiError::MissingApiKey);
    }

    let prompt = format!(
        "Suggest restrained visual references for a believable, spacious lived-in home in {location} during {season}, \
         to appear behind someone on a webcam call. Return ambiance, items and keywords. \
         Ambiance: briefly describe plausible residential architecture, materials and seasonal vegetation. \
         Items: at most three appealing, comfortable furnishings or subtle local details placed in the middle or far background, chosen to work together. \
         Keywords: at most four material, colour or architectural cues. \
         Describe only visible features. Do not prescribe time of day, sunlight, lighting or camera position; \
         these are specified separately by the image prompt. Avoid tourist attractions, landmark collections, \
         food close-ups, souvenir displays, people, smoke and references to sounds or smells. \
         Suggest a beautiful, welcoming home someone would want to spend time in: tactile materials, well-cared-for furnishings, \
         a comfortable place to linger and a plausible connection to the outdoors. Keep the choices attainable, personal and locally appropriate."
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
