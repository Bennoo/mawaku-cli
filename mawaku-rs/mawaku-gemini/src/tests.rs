use super::*;

#[test]
fn serialize_request_matches_expected_shape() {
    let request = PredictRequest::new("A cozy home office", DEFAULT_SAMPLE_COUNT, None);
    let value = serde_json::to_value(request).expect("serialize request");

    let expected = serde_json::json!({
        "instances": [{"prompt": "A cozy home office"}],
        "parameters": {"sampleCount": DEFAULT_SAMPLE_COUNT},
    });

    assert_eq!(value, expected);
}

#[test]
fn craft_prompt_builds_contextual_description() {
    let prompt = craft_prompt(
        "Base instructions.",
        "Lisbon, Portugal",
        Some("spring"),
        Some("golden hour"),
    );

    assert!(prompt.contains("Base instructions."));
    assert!(prompt.contains("Lisbon, Portugal"));
    assert!(prompt.contains("spring"));
    assert!(prompt.contains("golden hour"));
}

#[test]
fn craft_prompt_ignores_empty_inputs() {
    let prompt = craft_prompt("  ", "   ", Some("  "), Some(""));
    assert!(prompt.is_empty());
}

#[test]
fn empty_api_key_is_rejected() {
    let error = generate_image("   ", "workspace").expect_err("missing key");
    assert!(matches!(error, GeminiError::MissingApiKey));
}

#[test]
fn endpoint_uses_defaults() {
    let expected =
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-4.0-generate-001:predict";
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
