use design_providers::{
    credential_ref_for_provider, delete_provider_secret_with_store,
    read_provider_secret_with_store, replace_provider_secret_with_store, save_provider_with_store,
    MemoryCredentialStore, ProviderConfig, ProviderConfigView, ProviderKind, SERVICE_NAME,
};
use url::Url;
use uuid::Uuid;

fn provider_id() -> Uuid {
    Uuid::parse_str("6f904c54-2187-4b7b-85b7-95ab5bdf25aa").unwrap()
}

fn config() -> ProviderConfig {
    ProviderConfig {
        id: provider_id(),
        name: "OpenAI".to_owned(),
        kind: ProviderKind::OpenAi,
        base_url: Url::parse("https://api.openai.com/v1").unwrap(),
        model: "vision-model".to_owned(),
        credential_ref: String::new(),
    }
}

#[test]
fn provider_serialization_never_contains_the_secret() {
    let store = MemoryCredentialStore::default();
    let saved = save_provider_with_store(&store, config(), "sk-secret").unwrap();

    let json = serde_json::to_string(&saved).unwrap();

    assert!(!json.contains("sk-secret"));
    assert!(json.contains("credential_ref"));
}

#[test]
fn frontend_safe_view_exposes_only_credential_presence() {
    let store = MemoryCredentialStore::default();
    let saved = save_provider_with_store(&store, config(), "sk-secret").unwrap();
    let view = ProviderConfigView::from_config(&saved, true);

    let json = serde_json::to_string(&view).unwrap();

    assert!(view.has_credential);
    assert!(json.contains("has_credential"));
    assert!(!json.contains("sk-secret"));
    assert!(!json.contains("credential_ref"));
}

#[test]
fn memory_store_supports_create_read_replace_and_delete() {
    let store = MemoryCredentialStore::default();
    let saved = save_provider_with_store(&store, config(), "sk-original").unwrap();

    assert_eq!(
        read_provider_secret_with_store(&store, &saved).unwrap(),
        Some("sk-original".to_owned())
    );

    replace_provider_secret_with_store(&store, &saved, "sk-replacement").unwrap();
    assert_eq!(
        read_provider_secret_with_store(&store, &saved).unwrap(),
        Some("sk-replacement".to_owned())
    );

    delete_provider_secret_with_store(&store, &saved).unwrap();
    assert_eq!(
        read_provider_secret_with_store(&store, &saved).unwrap(),
        None
    );
}

#[test]
fn credential_ref_is_deterministic_and_uses_provider_id() {
    let credential_ref = credential_ref_for_provider(provider_id());

    assert_eq!(
        credential_ref,
        format!("keyring://{SERVICE_NAME}/provider:{}", provider_id())
    );
}
