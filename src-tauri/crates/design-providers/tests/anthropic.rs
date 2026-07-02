mod common;

use common::{MockResponse, MockServer};
use design_providers::{
    build_provider, AnalysisImage, AnalysisRequest, AnthropicProvider, MultimodalProvider,
    ProviderCapabilities, ProviderConfig, ProviderKind, SecretString,
};
use serde_json::{json, Value};
use url::Url;
use uuid::Uuid;

fn config(base_url: &str) -> ProviderConfig {
    config_with_kind(base_url, ProviderKind::Anthropic)
}

fn config_with_kind(base_url: &str, kind: ProviderKind) -> ProviderConfig {
    ProviderConfig {
        id: Uuid::parse_str("4b61dc18-d052-4a74-83b4-4515635ccf4a").unwrap(),
        name: "Anthropic".to_owned(),
        kind,
        base_url: Url::parse(base_url).unwrap(),
        model: "claude-vision".to_owned(),
        credential_ref: "keyring://test/provider".to_owned(),
    }
}

#[tokio::test]
async fn anthropic_preset_sends_base64_image_blocks_followed_by_text_without_schema_when_unsupported(
) {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"content":[{"type":"text","text":"{}"}]}"#,
    )]);
    let provider = AnthropicProvider::new(
        config(&server.base_url),
        SecretString::new("sk-ant"),
        reqwest::Client::new(),
    )
    .with_capabilities_override(ProviderCapabilities {
        image_input: true,
        structured_output: false,
    });

    provider
        .analyze(AnalysisRequest {
            model: "claude-vision".to_owned(),
            prompt: "analysis prompt".to_owned(),
            json_schema: json!({"type":"object"}),
            images: vec![AnalysisImage {
                media_type: "image/png".to_owned(),
                bytes: vec![1, 2, 3, 4],
            }],
        })
        .await
        .unwrap();

    let captured = server.single_request();
    assert_eq!(captured.path, "/messages");
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "sk-ant");
    assert!(captured.headers.contains_key("anthropic-version"));

    let body: Value = serde_json::from_str(&captured.body).unwrap();
    assert_eq!(body["model"], "claude-vision");
    assert_eq!(
        body["messages"][0]["content"][0],
        json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "AQIDBA=="
            }
        })
    );
    assert_eq!(
        body["messages"][0]["content"][1],
        json!({
            "type": "text",
            "text": "analysis prompt"
        })
    );
    assert!(body.get("tools").is_none());
    assert!(body.get("response_format").is_none());
}

#[tokio::test]
async fn factory_maps_anthropic_kind_to_anthropic_adapter() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"content":[{"type":"text","text":"{}"}]}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-ant"),
        reqwest::Client::new(),
    )
    .unwrap();

    provider
        .analyze(AnalysisRequest {
            model: "claude-vision".to_owned(),
            prompt: "analysis prompt".to_owned(),
            json_schema: json!({"type":"object"}),
            images: vec![],
        })
        .await
        .unwrap();

    assert_eq!(server.single_request().path, "/messages");
}

#[tokio::test]
async fn factory_maps_anthropic_compatible_kind_to_messages_adapter() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"content":[{"type":"text","text":"{}"}]}"#,
    )]);
    let provider = build_provider(
        &config_with_kind(&server.base_url, ProviderKind::AnthropicCompatible),
        SecretString::new("sk-compatible"),
        reqwest::Client::new(),
    )
    .unwrap();

    provider
        .analyze(AnalysisRequest {
            model: "third-party-claude-compatible".to_owned(),
            prompt: "analysis prompt".to_owned(),
            json_schema: json!({"type":"object"}),
            images: vec![AnalysisImage {
                media_type: "image/png".to_owned(),
                bytes: vec![9, 8, 7],
            }],
        })
        .await
        .unwrap();

    let captured = server.single_request();
    assert_eq!(captured.path, "/messages");
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "sk-compatible");
    assert!(captured.headers.contains_key("anthropic-version"));

    let body: Value = serde_json::from_str(&captured.body).unwrap();
    assert_eq!(body["model"], "third-party-claude-compatible");
    assert_eq!(body["messages"][0]["content"][0]["type"], "image");
    assert_eq!(body["messages"][0]["content"][1]["text"], "analysis prompt");
}
