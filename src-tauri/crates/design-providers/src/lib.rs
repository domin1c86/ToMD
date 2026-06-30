mod anthropic;
mod client;
pub mod config;
pub mod credentials;
mod error;
mod gemini;
mod openai;
mod openai_compatible;

pub use anthropic::AnthropicProvider;
pub use client::{
    build_provider, AnalysisImage, AnalysisRequest, MultimodalProvider, ProviderCapabilities,
    RawModelResponse, RequestLog, SecretString,
};
pub use config::{ProviderConfig, ProviderConfigView, ProviderKind};
pub use credentials::{
    credential_ref_for_provider, delete_provider_secret_with_store,
    read_provider_secret_with_store, replace_provider_secret_with_store, save_provider_with_store,
    CredentialStore, CredentialStoreError, KeyringCredentialStore, MemoryCredentialStore,
    SERVICE_NAME,
};
pub use error::ProviderError;
pub use gemini::GeminiProvider;
pub use openai::OpenAiProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
