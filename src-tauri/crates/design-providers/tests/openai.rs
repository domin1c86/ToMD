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
        id: Uuid::parse_str("0c3ef632-084f-4973-946a-bd133235329f").unwrap(),
        name: "OpenAI".to_owned(),
        kind: ProviderKind::OpenAi,
        base_url: Url::parse(base_url).unwrap(),
        model: "gpt-vision".to_owned(),
        credential_ref: "keyring://test/provider".to_owned(),
    }
}

#[tokio::test]
async fn openai_preset_uses_responses_api_with_input_images_and_json_output() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"id":"resp_1","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"{\"spacing\":[]}"}]}],"usage":{"input_tokens":10}}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-openai"),
        reqwest::Client::new(),
    )
    .unwrap();

    let response = provider
        .analyze(AnalysisRequest {
            model: "gpt-vision".to_owned(),
            prompt: "analysis prompt".to_owned(),
            json_schema: json!({"type":"object","properties":{"spacing":{"type":"array"}}}),
            images: vec![
                AnalysisImage {
                    media_type: "image/png".to_owned(),
                    bytes: vec![1, 2, 3, 4],
                },
                AnalysisImage {
                    media_type: "image/jpeg".to_owned(),
                    bytes: vec![5, 6, 7],
                },
            ],
        })
        .await
        .unwrap();

    assert_eq!(response.body, r#"{"spacing":[]}"#);

    let captured = server.single_request();
    assert_eq!(captured.path, "/responses");
    assert_eq!(
        captured.headers.get("authorization").unwrap(),
        "Bearer sk-openai"
    );

    let body: Value = serde_json::from_str(&captured.body).unwrap();
    assert_eq!(body["model"], "gpt-vision");
    assert_eq!(
        body["input"][0]["content"][0],
        json!({
            "type": "input_text",
            "text": "analysis prompt"
        })
    );
    assert_eq!(
        body["input"][0]["content"][1],
        json!({
            "type": "input_image",
            "image_url": "data:image/png;base64,AQIDBA=="
        })
    );
    assert_eq!(
        body["input"][0]["content"][2],
        json!({
            "type": "input_image",
            "image_url": "data:image/jpeg;base64,BQYH"
        })
    );
    assert_eq!(body["text"]["format"]["type"], "json_object");
    assert!(body["text"]["format"].get("schema").is_none());
    assert!(body["text"]["format"].get("strict").is_none());
}
