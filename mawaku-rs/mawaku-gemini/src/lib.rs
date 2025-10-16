use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_MODEL_VERSION: &str = "imagen-4.0-generate-001";
pub const DEFAULT_SAMPLE_COUNT: u32 = 2;
pub const DEFAULT_ASPECT_RATIO: &str = "16:9";

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
            parameters: Parameters { sample_count, aspect_ratio },
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
    let request_body = PredictRequest::new(prompt, DEFAULT_SAMPLE_COUNT, Some(DEFAULT_ASPECT_RATIO.to_string()));

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
        let expected = "https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict";
        assert_eq!(endpoint_url(), expected);
    }

    #[test]
    fn parses_prediction_payload_with_base64() {
        let json = r#"
        {
            "predictions": [
                {
                    "bytesBase64Encoded": "aGVsbG8=",
                    "mimeType": "image/png"
                }
            ]
        }
        "#;

        let response: PredictResponse = serde_json::from_str(json).expect("parse example response");
        assert_eq!(response.predictions.len(), 1);

        let prediction = &response.predictions[0];
        assert_eq!(prediction.bytes_base64_encoded.as_deref(), Some("aGVsbG8="));
        assert_eq!(prediction.mime_type.as_deref(), Some("image/png"));
    }
}
