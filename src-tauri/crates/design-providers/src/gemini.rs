use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::json;

use crate::{
    client::{
        base64_image, elapsed_ms, endpoint_url, parse_capabilities, raw_response, validate_model,
        validate_request, AnalysisRequest, MultimodalProvider, ProviderCapabilities,
        RawModelResponse, RequestLog, SecretString,
    },
    ProviderConfig, ProviderError,
};

/// Connection tests should fail fast instead of inheriting the long
/// analysis timeout from the shared HTTP client.
const TEST_CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

pub struct GeminiProvider {
    config: ProviderConfig,
    secret: SecretString,
    client: reqwest::Client,
    capabilities_override: Option<ProviderCapabilities>,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig, secret: SecretString, client: reqwest::Client) -> Self {
        Self {
            config,
            secret,
            client,
            capabilities_override: None,
        }
    }

    pub fn with_capabilities_override(mut self, capabilities: ProviderCapabilities) -> Self {
        self.capabilities_override = Some(capabilities);
        self
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities_override
            .clone()
            .unwrap_or_else(ProviderCapabilities::full_multimodal_json_schema)
    }
}

#[async_trait]
impl MultimodalProvider for GeminiProvider {
    async fn test_connection(&self) -> Result<ProviderCapabilities, ProviderError> {
        validate_model(&self.config.model)?;
        if let Some(capabilities) = &self.capabilities_override {
            return Ok(capabilities.clone());
        }

        let url = endpoint_url(
            &self.config.base_url,
            &format!("models/{}", self.config.model),
        )?;
        let response = self
            .client
            .get(url)
            .timeout(TEST_CONNECTION_TIMEOUT)
            .header("x-goog-api-key", self.secret.expose_secret())
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;

        parse_capabilities(response, self.capabilities()).await
    }

    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError> {
        validate_request(&request, &self.capabilities())?;

        let parts = std::iter::once(json!({
            "text": request.prompt,
        }))
        .chain(request.images.iter().map(|image| {
            json!({
                "inline_data": {
                    "mime_type": image.media_type,
                    "data": base64_image(image),
                },
            })
        }))
        .collect::<Vec<_>>();

        let body = json!({
            "contents": [{
                "role": "user",
                "parts": parts,
            }],
            "generationConfig": {
                "response_mime_type": "application/json",
            },
        });

        let started_at = Instant::now();
        let response = self
            .client
            .post(endpoint_url(
                &self.config.base_url,
                &format!("models/{}:generateContent", request.model),
            )?)
            .header("x-goog-api-key", self.secret.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;

        let raw = raw_response(response).await?;
        let _log = RequestLog {
            provider_id: self.config.id,
            model: self.config.model.clone(),
            image_count: request.images.len(),
            duration_ms: elapsed_ms(started_at),
            status_code: Some(raw.status_code),
            request_id: raw.request_id.clone(),
        };

        let text = crate::extract::gemini_candidates_text(&raw.body)?;
        Ok(RawModelResponse { body: text, ..raw })
    }
}
