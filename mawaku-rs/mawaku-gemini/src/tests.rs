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
    assert_eq!(image_endpoint_url(), expected);
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

#[test]
fn text_request_with_schema_serializes_correctly() {
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

    let request = TextRequest::with_schema("Test prompt", generation_config);
    let value = serde_json::to_value(request).expect("serialize request");

    assert!(value["contents"].is_array());
    assert_eq!(value["contents"][0]["parts"][0]["text"], "Test prompt");
    assert_eq!(
        value["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(
        value["generationConfig"]["responseSchema"]["type"],
        "OBJECT"
    );
    assert!(value["generationConfig"]["responseSchema"]["properties"]["ambiance"].is_object());
}

#[test]
fn place_description_parses_from_json() {
    let json = r#"
    {
        "ambiance": "Warm and inviting",
        "items": ["wooden chair", "bookshelf", "plant"],
        "keywords": ["cozy", "rustic", "natural"]
    }
    "#;

    let description: PlaceDescription =
        serde_json::from_str(json).expect("parse place description");
    assert_eq!(description.ambiance, "Warm and inviting");
    assert_eq!(description.items.len(), 3);
    assert_eq!(description.keywords.len(), 3);
    assert!(description.items.contains(&"wooden chair".to_string()));
    assert!(description.keywords.contains(&"cozy".to_string()));
}

#[test]
fn place_description_displays_formatted() {
    let description = PlaceDescription {
        ambiance: "Warm and inviting".to_string(),
        items: vec!["chair".to_string(), "table".to_string()],
        keywords: vec!["cozy".to_string(), "rustic".to_string()],
    };

    let formatted = format!("{}", description);
    assert!(formatted.contains("Ambiance: Warm and inviting"));
    assert!(formatted.contains("Items: chair, table"));
    assert!(formatted.contains("Keywords: cozy, rustic"));
}
