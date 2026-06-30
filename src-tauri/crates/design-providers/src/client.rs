use std::{
    fmt,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{header::HeaderMap, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use crate::{
    AnthropicProvider, GeminiProvider, OpenAiCompatibleProvider, OpenAiProvider, ProviderConfig,
    ProviderError, ProviderKind,
};

#[async_trait]
pub trait MultimodalProvider: Send + Sync {
    async fn test_connection(&self) -> Result<ProviderCapabilities, ProviderError>;
    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub image_input: bool,
    pub structured_output: bool,
}

impl ProviderCapabilities {
    pub(crate) fn full_multimodal_json_schema() -> Self {
        Self {
            image_input: true,
            structured_output: true,
        }
    }

    pub(crate) fn image_only() -> Self {
        Self {
            image_input: true,
            structured_output: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisImage {
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisRequest {
    pub model: String,
    pub prompt: String,
    pub json_schema: Value,
    pub images: Vec<AnalysisImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawModelResponse {
    pub body: String,
    pub status_code: u16,
    pub request_id: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    pub(crate) fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RequestLog {
    pub provider_id: Uuid,
    pub model: String,
    pub image_count: usize,
    pub duration_ms: u128,
    pub status_code: Option<u16>,
    pub request_id: Option<String>,
}

impl fmt::Debug for RequestLog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RequestLog")
            .field("provider_id", &self.provider_id)
            .field("model", &self.model)
            .field("image_count", &self.image_count)
            .field("duration_ms", &self.duration_ms)
            .field("status_code", &self.status_code)
            .field("request_id", &self.request_id)
            .finish()
    }
}

pub fn build_provider(
    config: &ProviderConfig,
    secret: SecretString,
    client: reqwest::Client,
) -> Result<Box<dyn MultimodalProvider>, ProviderError> {
    if config.model.trim().is_empty() {
        return Err(ProviderError::MissingModel);
    }

    let config = config.clone();
    let provider: Box<dyn MultimodalProvider> = match config.kind {
        ProviderKind::OpenAi => Box::new(OpenAiProvider::new(config, secret, client)),
        ProviderKind::Anthropic => Box::new(AnthropicProvider::new(config, secret, client)),
        ProviderKind::Gemini => Box::new(GeminiProvider::new(config, secret, client)),
        ProviderKind::OpenAiCompatible => {
            Box::new(OpenAiCompatibleProvider::new(config, secret, client))
        }
    };

    Ok(provider)
}

pub(crate) fn validate_request(
    request: &AnalysisRequest,
    capabilities: &ProviderCapabilities,
) -> Result<(), ProviderError> {
    if request.model.trim().is_empty() {
        return Err(ProviderError::MissingModel);
    }

    if !request.images.is_empty() && !capabilities.image_input {
        return Err(ProviderError::CapabilityMismatch);
    }

    if !request.json_schema.is_null() && !capabilities.structured_output {
        return Err(ProviderError::CapabilityMismatch);
    }

    Ok(())
}

pub(crate) fn validate_model(model: &str) -> Result<(), ProviderError> {
    if model.trim().is_empty() {
        Err(ProviderError::MissingModel)
    } else {
        Ok(())
    }
}

pub(crate) fn image_data_url(image: &AnalysisImage) -> String {
    format!(
        "data:{};base64,{}",
        image.media_type,
        STANDARD.encode(&image.bytes)
    )
}

pub(crate) fn base64_image(image: &AnalysisImage) -> String {
    STANDARD.encode(&image.bytes)
}

pub(crate) fn endpoint_url(base_url: &Url, endpoint: &str) -> Result<Url, ProviderError> {
    let mut base = base_url.as_str().trim_end_matches('/').to_owned();
    base.push('/');
    base.push_str(endpoint.trim_start_matches('/'));
    Url::parse(&base).map_err(|error| ProviderError::InvalidResponse {
        message: format!("invalid provider URL: {error}"),
    })
}

pub(crate) async fn raw_response(response: Response) -> Result<RawModelResponse, ProviderError> {
    let status = response.status();
    let status_code = status.as_u16();
    let request_id = request_id(response.headers());

    if !status.is_success() {
        return Err(ProviderError::from_status(status));
    }

    let body = response.text().await.map_err(ProviderError::from_reqwest)?;

    Ok(RawModelResponse {
        body,
        status_code,
        request_id,
    })
}

pub(crate) fn request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .or_else(|| headers.get("request-id"))
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub(crate) async fn parse_capabilities(
    response: Response,
) -> Result<ProviderCapabilities, ProviderError> {
    let status = response.status();
    if !status.is_success() {
        return Err(ProviderError::from_status(status));
    }

    let body: Value = response.json().await.map_err(ProviderError::from_reqwest)?;
    let capabilities = body.get("capabilities").unwrap_or(&body);

    Ok(ProviderCapabilities {
        image_input: capabilities
            .get("image_input")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        structured_output: capabilities
            .get("structured_output")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub(crate) fn elapsed_ms(started_at: Instant) -> u128 {
    duration_ms(started_at.elapsed())
}

fn duration_ms(duration: Duration) -> u128 {
    duration.as_millis()
}
