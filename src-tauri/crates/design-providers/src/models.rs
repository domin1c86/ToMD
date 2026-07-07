use std::time::Duration;

use serde_json::Value;
use url::Url;

use crate::{
    client::{endpoint_url, SecretString},
    ProviderError, ProviderKind,
};

/// Model listing is an interactive lookup; fail fast like connection tests.
const MODEL_LIST_TIMEOUT: Duration = Duration::from_secs(30);

/// Fetches the model ids offered by the endpoint (`GET {base}/models`),
/// normalized across provider shapes.
pub async fn list_models(
    kind: ProviderKind,
    base_url: &Url,
    secret: &SecretString,
    client: &reqwest::Client,
) -> Result<Vec<String>, ProviderError> {
    let url = endpoint_url(base_url, "models")?;
    let request = client.get(url).timeout(MODEL_LIST_TIMEOUT);
    let request = match kind {
        ProviderKind::OpenAi | ProviderKind::OpenAiCompatible => {
            request.bearer_auth(secret.expose_secret())
        }
        ProviderKind::Anthropic | ProviderKind::AnthropicCompatible => request
            .header("x-api-key", secret.expose_secret())
            .header("anthropic-version", "2023-06-01"),
        ProviderKind::Gemini => request.header("x-goog-api-key", secret.expose_secret()),
    };

    let response = request.send().await.map_err(ProviderError::from_reqwest)?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ProviderError::from_status_and_body(status, &body));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|_| ProviderError::InvalidResponse {
            message: "model list response was not valid JSON".to_owned(),
        })?;

    let mut models = parse_model_ids(kind, &body);
    if models.is_empty() {
        return Err(ProviderError::InvalidResponse {
            message: "model list response contained no models".to_owned(),
        });
    }
    models.sort();
    models.dedup();
    Ok(models)
}

fn parse_model_ids(kind: ProviderKind, body: &Value) -> Vec<String> {
    match kind {
        // Gemini: {"models":[{"name":"models/gemini-..."}]}
        ProviderKind::Gemini => body
            .get("models")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| model.get("name").and_then(Value::as_str))
                    .map(|name| name.strip_prefix("models/").unwrap_or(name).to_owned())
                    .collect()
            })
            .unwrap_or_default(),
        // OpenAI, Anthropic, and compatibles: {"data":[{"id":"..."}]}
        _ => body
            .get("data")
            .and_then(Value::as_array)
            .map(|models| {
                models
                    .iter()
                    .filter_map(|model| model.get("id").and_then(Value::as_str))
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    }
}
