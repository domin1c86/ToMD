mod common;

use common::{MockResponse, MockServer};
use design_providers::{list_models, ProviderError, ProviderKind, SecretString};
use url::Url;

async fn fetch(kind: ProviderKind, body: &str, status: u16) -> Result<Vec<String>, ProviderError> {
    let server = MockServer::spawn(vec![MockResponse::json(status, body)]);
    let url = Url::parse(&server.base_url).unwrap();
    list_models(
        kind,
        &url,
        &SecretString::new("sk-secret"),
        &reqwest::Client::new(),
    )
    .await
}

#[tokio::test]
async fn lists_openai_style_models_with_bearer_auth() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"object":"list","data":[{"id":"vision-large"},{"id":"vision-mini"}]}"#,
    )]);
    let url = Url::parse(&server.base_url).unwrap();

    let models = list_models(
        ProviderKind::OpenAiCompatible,
        &url,
        &SecretString::new("sk-secret"),
        &reqwest::Client::new(),
    )
    .await
    .unwrap();

    assert_eq!(models, vec!["vision-large".to_owned(), "vision-mini".to_owned()]);
    let captured = server.single_request();
    assert_eq!(captured.method, "GET");
    assert_eq!(captured.path, "/models");
    assert_eq!(captured.headers.get("authorization").unwrap(), "Bearer sk-secret");
}

#[tokio::test]
async fn lists_anthropic_models_with_api_key_header() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"data":[{"id":"claude-vision","display_name":"Claude"}]}"#,
    )]);
    let url = Url::parse(&server.base_url).unwrap();

    let models = list_models(
        ProviderKind::Anthropic,
        &url,
        &SecretString::new("sk-ant"),
        &reqwest::Client::new(),
    )
    .await
    .unwrap();

    assert_eq!(models, vec!["claude-vision".to_owned()]);
    let captured = server.single_request();
    assert_eq!(captured.headers.get("x-api-key").unwrap(), "sk-ant");
    assert!(captured.headers.contains_key("anthropic-version"));
}

#[tokio::test]
async fn lists_gemini_models_and_strips_the_models_prefix() {
    let server = MockServer::spawn(vec![MockResponse::json(
        200,
        r#"{"models":[{"name":"models/gemini-vision"},{"name":"models/gemini-flash"}]}"#,
    )]);
    let url = Url::parse(&server.base_url).unwrap();

    let models = list_models(
        ProviderKind::Gemini,
        &url,
        &SecretString::new("gm-secret"),
        &reqwest::Client::new(),
    )
    .await
    .unwrap();

    assert_eq!(models, vec!["gemini-flash".to_owned(), "gemini-vision".to_owned()]);
    assert_eq!(
        server.single_request().headers.get("x-goog-api-key").unwrap(),
        "gm-secret"
    );
}

#[tokio::test]
async fn maps_auth_failures_and_rejects_empty_lists() {
    let error = fetch(ProviderKind::OpenAi, r#"{"error":"nope"}"#, 401)
        .await
        .unwrap_err();
    assert_eq!(error, ProviderError::Authentication);

    let error = fetch(ProviderKind::OpenAi, r#"{"data":[]}"#, 200)
        .await
        .unwrap_err();
    assert!(matches!(error, ProviderError::InvalidResponse { .. }));

    let error = fetch(ProviderKind::OpenAi, "not json sk-leak", 200)
        .await
        .unwrap_err();
    assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    assert!(!format!("{error}").contains("sk-leak"));
}
