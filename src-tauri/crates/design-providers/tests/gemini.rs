mod common;

use common::{MockResponse, MockServer};
use design_providers::{
    build_provider, AnalysisImage, AnalysisRequest, ProviderConfig, ProviderKind, SecretString,
};
use serde_json::{json, Value};
use url::Url;
use uuid::Uuid;

fn config(base_url: &str) -> ProviderConfig {
    ProviderConfig {
        id: Uuid::parse_str("afdb3097-9082-43f4-8e37-12dc65ec82e0").unwrap(),
        name: "Gemini".to_owned(),
        kind: ProviderKind::Gemini,
        base_url: Url::parse(base_url).unwrap(),
        model: "gemini-vision".to_owned(),
        credential_ref: "keyring://test/provider".to_owned(),
    }
}

#[tokio::test]
async fn gemini_preset_uses_generate_content_with_inline_data_and_response_schema() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"candidates":[{"content":{"parts":[{"text":"{}"}]}}]}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("gemini-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    provider
        .analyze(AnalysisRequest {
            model: "gemini-vision".to_owned(),
            prompt: "analysis prompt".to_owned(),
            json_schema: json!({"type":"object","properties":{"typography":{"type":"array"}}}),
            images: vec![AnalysisImage {
                media_type: "image/png".to_owned(),
                bytes: vec![1, 2, 3, 4],
            }],
        })
        .await
        .unwrap();

    let captured = server.single_request();
    assert_eq!(captured.path, "/models/gemini-vision:generateContent");
    assert_eq!(
        captured.headers.get("authorization").unwrap(),
        "Bearer gemini-secret"
    );

    let body: Value = serde_json::from_str(&captured.body).unwrap();
    assert_eq!(
        body["contents"][0]["parts"][0],
        json!({
            "text": "analysis prompt"
        })
    );
    assert_eq!(
        body["contents"][0]["parts"][1],
        json!({
            "inline_data": {
                "mime_type": "image/png",
                "data": "AQIDBA=="
            }
        })
    );
    assert_eq!(
        body["generationConfig"]["response_mime_type"],
        "application/json"
    );
    assert_eq!(
        body["generationConfig"]["response_schema"],
        json!({"type":"object","properties":{"typography":{"type":"array"}}})
    );
}
