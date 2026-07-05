use std::time::Instant;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    client::{
        base64_image, elapsed_ms, endpoint_url, parse_capabilities, raw_response, validate_model,
        AnalysisRequest, MultimodalProvider, ProviderCapabilities, RawModelResponse, RequestLog,
        SecretString,
    },
    ProviderConfig, ProviderError,
};

pub struct AnthropicProvider {
    config: ProviderConfig,
    secret: SecretString,
    client: reqwest::Client,
    capabilities_override: Option<ProviderCapabilities>,
}

impl AnthropicProvider {
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
            .unwrap_or_else(ProviderCapabilities::image_only)
    }
}

#[async_trait]
impl MultimodalProvider for AnthropicProvider {
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
            .header("x-api-key", self.secret.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(ProviderError::from_reqwest)?;

        parse_capabilities(response, self.capabilities()).await
    }

    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError> {
        validate_model(&request.model)?;
        let capabilities = self.capabilities();
        if !request.images.is_empty() && !capabilities.image_input {
            return Err(ProviderError::CapabilityMismatch);
        }

        let mut content = request
            .images
            .iter()
            .map(|image| {
                json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": image.media_type,
                        "data": base64_image(image),
                    },
                })
            })
            .collect::<Vec<_>>();
        content.push(json!({
            "type": "text",
            "text": request.prompt,
        }));

        let mut body = json!({
            "model": request.model,
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": content,
            }],
        });

        if capabilities.structured_output && !request.json_schema.is_null() {
            body["tools"] = json!([{
                "name": "emit_design_spec",
                "description": "Return the design specification as structured JSON.",
                "input_schema": request.json_schema,
            }]);
            body["tool_choice"] = json!({
                "type": "tool",
                "name": "emit_design_spec",
            });
        }

        let started_at = Instant::now();
        let response = self
            .client
            .post(endpoint_url(&self.config.base_url, "messages")?)
            .header("x-api-key", self.secret.expose_secret())
            .header("anthropic-version", "2023-06-01")
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

        let text = crate::extract::anthropic_messages_text(&raw.body)?;
        Ok(RawModelResponse { body: text, ..raw })
    }
}
