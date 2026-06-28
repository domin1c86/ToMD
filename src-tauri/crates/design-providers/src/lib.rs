pub mod config;
pub mod credentials;

pub use config::{ProviderConfig, ProviderConfigView, ProviderKind};
pub use credentials::{
    credential_ref_for_provider, delete_provider_secret_with_store,
    read_provider_secret_with_store, replace_provider_secret_with_store, save_provider_with_store,
    CredentialStore, CredentialStoreError, KeyringCredentialStore, MemoryCredentialStore,
    SERVICE_NAME,
};
