mod common;

use std::time::Duration;

use common::{MockResponse, MockServer};
use design_providers::{
    build_provider, AnalysisImage, AnalysisRequest, MultimodalProvider, OpenAiCompatibleProvider,
    ProviderCapabilities, ProviderConfig, ProviderError, ProviderKind, SecretString,
};
use serde_json::{json, Value};
use url::Url;
use uuid::Uuid;

fn config(base_url: &str) -> ProviderConfig {
    ProviderConfig {
        id: Uuid::parse_str("6f904c54-2187-4b7b-85b7-95ab5bdf25aa").unwrap(),
        name: "Local compatible".to_owned(),
        kind: ProviderKind::OpenAiCompatible,
        base_url: Url::parse(base_url).unwrap(),
        model: "vision-model".to_owned(),
        credential_ref: "keyring://test/provider".to_owned(),
    }
}

fn request() -> AnalysisRequest {
    AnalysisRequest {
        model: "vision-model".to_owned(),
        prompt: "analysis prompt".to_owned(),
        json_schema: json!({
            "type": "object",
            "properties": { "colors": { "type": "array" } },
            "required": ["colors"]
        }),
        images: vec![AnalysisImage {
            media_type: "image/png".to_owned(),
            bytes: vec![1, 2, 3, 4],
        }],
    }
}

#[tokio::test]
async fn sends_chat_completions_shape_with_data_url_and_strict_json_schema() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"choices":[{"message":{"content":"{\"colors\":[]}"}}]}"#,
    )
    .with_header("x-request-id", "req-compatible")]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    let response = provider.analyze(request()).await.unwrap();

    let captured = server.single_request();
    assert_eq!(captured.method, "POST");
    assert_eq!(captured.path, "/chat/completions");
    assert_eq!(
        captured.headers.get("authorization").unwrap(),
        "Bearer sk-secret"
    );
    assert_eq!(response.request_id.as_deref(), Some("req-compatible"));
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("choices"));

    let body: Value = serde_json::from_str(&captured.body).unwrap();
    assert_eq!(body["model"], "vision-model");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(
        body["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "analysis prompt"
        })
    );
    assert_eq!(
        body["messages"][0]["content"][1],
        json!({
            "type": "image_url",
            "image_url": { "url": "data:image/png;base64,AQIDBA==" }
        })
    );
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(
        body["response_format"]["json_schema"]["name"],
        "design_spec"
    );
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        body["response_format"]["json_schema"]["schema"],
        request().json_schema
    );
}

#[tokio::test]
async fn maps_authentication_and_rate_limit_statuses() {
    for (status, expected) in [
        (401, ProviderError::Authentication),
        (429, ProviderError::RateLimited),
    ] {
        let server = MockServer::spawn(vec![MockResponse::json(status, r#"{"error":"nope"}"#)]);
        let provider = build_provider(
            &config(&server.base_url),
            SecretString::new("sk-secret"),
            reqwest::Client::new(),
        )
        .unwrap();

        let error = provider.analyze(request()).await.unwrap_err();

        assert_eq!(error, expected);
    }
}

#[tokio::test]
async fn test_connection_fetches_configured_model_capabilities() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"capabilities":{"image_input":true,"structured_output":true}}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    let capabilities = provider.test_connection().await.unwrap();

    assert_eq!(
        capabilities,
        ProviderCapabilities {
            image_input: true,
            structured_output: true,
        }
    );
    let captured = server.single_request();
    assert_eq!(captured.method, "GET");
    assert_eq!(captured.path, "/models/vision-model");
}

#[tokio::test]
async fn test_connection_defaults_to_adapter_capabilities_when_metadata_has_no_capabilities() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"id":"vision-model","object":"model"}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    let capabilities = provider.test_connection().await.unwrap();

    assert_eq!(
        capabilities,
        ProviderCapabilities {
            image_input: true,
            structured_output: true,
        }
    );
}

#[tokio::test]
async fn test_connection_maps_malformed_capability_json_to_invalid_response() {
    let server = MockServer::spawn(vec![MockResponse::json(200, "not json")]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    let error = provider.test_connection().await.unwrap_err();

    assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    assert!(!format!("{error}").contains("not json"));
}

#[tokio::test]
async fn test_connection_rejects_non_boolean_capability_fields() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"capabilities":{"image_input":"false","structured_output":true}}"#,
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .unwrap();

    let error = provider.test_connection().await.unwrap_err();

    assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    assert!(format!("{error}").contains("image_input"));
    assert!(!format!("{error}").contains("structured_output"));
    assert!(!format!("{error}").contains("false"));
}

#[tokio::test]
async fn maps_reqwest_timeout_to_provider_timeout() {
    let server = MockServer::spawn(vec![MockResponse::delayed_json(
        200,
        r#"{"choices":[]}"#,
        Duration::from_millis(200),
    )]);
    let provider = build_provider(
        &config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::builder()
            .timeout(Duration::from_millis(20))
            .build()
            .unwrap(),
    )
    .unwrap();

    let error = provider.analyze(request()).await.unwrap_err();

    assert_eq!(error, ProviderError::Timeout);
}

#[tokio::test]
async fn rejects_schema_request_when_structured_outputs_are_unsupported() {
    let server = MockServer::spawn(vec![MockResponse::json(200, r#"{"choices":[]}"#)]);
    let provider = OpenAiCompatibleProvider::new(
        config(&server.base_url),
        SecretString::new("sk-secret"),
        reqwest::Client::new(),
    )
    .with_capabilities_override(ProviderCapabilities {
        image_input: true,
        structured_output: false,
    });

    let error = provider.analyze(request()).await.unwrap_err();

    assert_eq!(error, ProviderError::CapabilityMismatch);
    assert!(server.requests().is_empty());
}

#[test]
fn secret_string_and_request_log_debug_are_redacted() {
    let secret = SecretString::new("sk-secret");
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(format!("{secret:?}"), "[REDACTED]");

    let log = design_providers::RequestLog {
        provider_id: Uuid::parse_str("6f904c54-2187-4b7b-85b7-95ab5bdf25aa").unwrap(),
        model: "vision-model".to_owned(),
        image_count: 1,
        duration_ms: 12,
        status_code: Some(200),
        request_id: Some("req-compatible".to_owned()),
    };
    let rendered = format!("{log:?}");

    assert!(rendered.contains("vision-model"));
    assert!(rendered.contains("image_count"));
    assert!(!rendered.contains("sk-secret"));
    assert!(!rendered.contains("analysis prompt"));
    assert!(!rendered.contains("AQIDBA=="));
    assert!(!rendered.contains("choices"));
}

#[test]
fn sensitive_request_and_response_debug_are_redacted() {
    let request = request();
    let request_debug = format!("{request:?}");

    assert!(request_debug.contains("vision-model"));
    assert!(request_debug.contains("image_count"));
    assert!(!request_debug.contains("analysis prompt"));
    assert!(!request_debug.contains("AQIDBA=="));
    assert!(!request_debug.contains("[1, 2, 3, 4]"));
    assert!(!request_debug.contains("colors"));

    let image_debug = format!("{:?}", request.images[0]);
    assert!(image_debug.contains("image/png"));
    assert!(image_debug.contains("byte_len"));
    assert!(!image_debug.contains("[1, 2, 3, 4]"));
    assert!(!image_debug.contains("AQIDBA=="));

    let raw = design_providers::RawModelResponse {
        body: r#"{"secret":"provider body"}"#.to_owned(),
        status_code: 200,
        request_id: Some("req-compatible".to_owned()),
    };
    let raw_debug = format!("{raw:?}");
    assert!(raw_debug.contains("status_code"));
    assert!(raw_debug.contains("req-compatible"));
    assert!(!raw_debug.contains("provider body"));
    assert!(!raw_debug.contains("secret"));
}
