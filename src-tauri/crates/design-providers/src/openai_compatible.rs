use std::time::Instant;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    client::{
        elapsed_ms, endpoint_url, image_data_url, parse_capabilities, raw_response, validate_model,
        validate_request, AnalysisRequest, MultimodalProvider, ProviderCapabilities,
        RawModelResponse, RequestLog, SecretString,
    },
    ProviderConfig, ProviderError,
};

pub struct OpenAiCompatibleProvider {
    config: ProviderConfig,
    secret: SecretString,
    client: reqwest::Client,
    capabilities_override: Option<ProviderCapabilities>,
}

impl OpenAiCompatibleProvider {
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
impl MultimodalProvider for OpenAiCompatibleProvider {
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
            .bearer_auth(self.secret.expose_secret())
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;

        parse_capabilities(response, self.capabilities()).await
    }

    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError> {
        validate_request(&request, &self.capabilities())?;

        let content = std::iter::once(json!({
            "type": "text",
            "text": request.prompt,
        }))
        .chain(request.images.iter().map(|image| {
            json!({
                "type": "image_url",
                "image_url": {
                    "url": image_data_url(image),
                },
            })
        }))
        .collect::<Vec<_>>();

        let body = json!({
            "model": request.model,
            "messages": [{
                "role": "user",
                "content": content,
            }],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "design_spec",
                    "strict": true,
                    "schema": request.json_schema,
                },
            },
        });

        let started_at = Instant::now();
        let response = self
            .client
            .post(endpoint_url(&self.config.base_url, "chat/completions")?)
            .bearer_auth(self.secret.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;

        let raw = raw_response(response).await?;
        let _log = RequestLog {
            provider_id: self.config.id,
            model: body["model"].as_str().unwrap_or_default().to_owned(),
            image_count: request.images.len(),
            duration_ms: elapsed_ms(started_at),
            status_code: Some(raw.status_code),
            request_id: raw.request_id.clone(),
        };

        let text = crate::extract::chat_completions_text(&raw.body)?;
        Ok(RawModelResponse { body: text, ..raw })
    }
}
