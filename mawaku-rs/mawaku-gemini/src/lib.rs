use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const DEFAULT_LOCATION: &str = "europe-west2";
pub const DEFAULT_PROJECT_ID: &str = "fake-cloud-project";
pub const DEFAULT_MODEL_VERSION: &str = "imagen-4.0-generate-001";
pub const DEFAULT_SAMPLE_COUNT: u32 = 2;

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
    pub predictions: Vec<Value>,
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
}

impl<'a> PredictRequest<'a> {
    fn new(prompt: &'a str, sample_count: u32) -> Self {
        Self {
            instances: vec![Instance { prompt }],
            parameters: Parameters { sample_count },
        }
    }
}

fn endpoint_url(project_id: &str, location: &str) -> String {
    format!(
        "https://{location}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{location}/publishers/google/models/{model_version}:predict",
        location = location,
        project_id = project_id,
        model_version = DEFAULT_MODEL_VERSION
    )
}

/// Submit an image generation request to Gemini's Imagen 4 API.
///
/// The request is dispatched to the default `europe-west2` location using a fake
/// project identifier to keep the initial implementation simple. Future
/// iterations can expose configuration hooks for these values.
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
    let url = endpoint_url(DEFAULT_PROJECT_ID, DEFAULT_LOCATION);
    let request_body = PredictRequest::new(prompt, DEFAULT_SAMPLE_COUNT);

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()?;

    let response = response.error_for_status()?;
    let parsed = response.json::<PredictResponse>()?;
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_request_matches_expected_shape() {
        let request = PredictRequest::new("A cozy home office", DEFAULT_SAMPLE_COUNT);
        let value = serde_json::to_value(request).expect("serialize request");

        let expected = serde_json::json!({
            "instances": [{"prompt": "A cozy home office"}],
            "parameters": {"sampleCount": DEFAULT_SAMPLE_COUNT},
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn empty_api_key_is_rejected() {
        let error = generate_image("   ", "workspace").expect_err("missing key");
        assert!(matches!(error, GeminiError::MissingApiKey));
    }

    #[test]
    fn endpoint_uses_defaults() {
        let expected = "https://europe-west2-aiplatform.googleapis.com/v1/projects/fake-cloud-project/locations/europe-west2/publishers/google/models/imagen-4.0-generate-001:predict";
        assert_eq!(endpoint_url(DEFAULT_PROJECT_ID, DEFAULT_LOCATION), expected);
    }
}
