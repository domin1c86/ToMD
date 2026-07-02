use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: Uuid,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: Url,
    pub model: String,
    pub credential_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    OpenAiCompatible,
    AnthropicCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfigView {
    pub id: Uuid,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: Url,
    pub model: String,
    pub has_credential: bool,
}

impl ProviderConfigView {
    pub fn from_config(config: &ProviderConfig, has_credential: bool) -> Self {
        Self {
            id: config.id,
            name: config.name.clone(),
            kind: config.kind,
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            has_credential,
        }
    }
}
