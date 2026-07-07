use chrono::{DateTime, Utc};
use design_providers::{
    build_provider, credential_ref_for_provider, delete_provider_secret_with_store, list_models,
    read_provider_secret_with_store, replace_provider_secret_with_store, save_provider_with_store,
    ProviderCapabilities, ProviderConfig, ProviderConfigView, ProviderKind, SecretString,
};
use design_storage::open_connection;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use url::Url;
use uuid::Uuid;

use crate::state::AppState;

use super::{command_error, parse_uuid, CommandResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderInput {
    provider_id: Option<String>,
    name: String,
    kind: ProviderKind,
    base_url: Url,
    model: String,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderIdInput {
    provider_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderView {
    id: Uuid,
    name: String,
    kind: ProviderKind,
    base_url: Url,
    model: String,
    has_credential: bool,
}

impl From<ProviderConfigView> for ProviderView {
    fn from(view: ProviderConfigView) -> Self {
        Self {
            id: view.id,
            name: view.name,
            kind: view.kind,
            base_url: view.base_url,
            model: view.model,
            has_credential: view.has_credential,
        }
    }
}

#[tauri::command]
pub async fn list_providers(state: State<'_, AppState>) -> CommandResult<Vec<ProviderView>> {
    let db_path = state.db_path.clone();
    let store = state.credential_store;
    tauri::async_runtime::spawn_blocking(move || {
        let configs = list_provider_configs(&db_path)?;
        configs
            .into_iter()
            .map(|config| {
                let has_credential = read_provider_secret_with_store(&store, &config)
                    .map_err(command_error)?
                    .is_some();
                Ok(ProviderView::from(ProviderConfigView::from_config(
                    &config,
                    has_credential,
                )))
            })
            .collect()
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn save_provider(
    state: State<'_, AppState>,
    input: SaveProviderInput,
) -> CommandResult<ProviderView> {
    let provider_id = input
        .provider_id
        .as_deref()
        .map(|id| parse_uuid(id, "providerId"))
        .transpose()?
        .unwrap_or_else(Uuid::new_v4);
    let store = state.credential_store;
    let db_path = state.db_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let mut config = ProviderConfig {
            id: provider_id,
            name: input.name,
            kind: input.kind,
            base_url: input.base_url,
            model: input.model,
            credential_ref: credential_ref_for_provider(provider_id),
        };

        let existing = get_provider_config(&db_path, provider_id).ok();
        match (existing.as_ref(), input.api_key.as_deref()) {
            (None, Some(secret)) => {
                config = save_provider_with_store(&store, config, secret).map_err(command_error)?;
            }
            (None, None) => return Err("apiKey is required for a new provider".to_owned()),
            (Some(existing), Some(secret)) => {
                config.credential_ref = existing.credential_ref.clone();
                replace_provider_secret_with_store(&store, &config, secret)
                    .map_err(command_error)?;
            }
            (Some(existing), None) => {
                config.credential_ref = existing.credential_ref.clone();
            }
        }

        upsert_provider_config(&db_path, &config)?;
        let has_credential = read_provider_secret_with_store(&store, &config)
            .map_err(command_error)?
            .is_some();
        Ok(ProviderView::from(ProviderConfigView::from_config(
            &config,
            has_credential,
        )))
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn delete_provider(
    state: State<'_, AppState>,
    input: ProviderIdInput,
) -> CommandResult<()> {
    let provider_id = parse_uuid(&input.provider_id, "providerId")?;
    let db_path = state.db_path.clone();
    let store = state.credential_store;
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(config) = get_provider_config(&db_path, provider_id) {
            delete_provider_secret_with_store(&store, &config).map_err(command_error)?;
        }
        open_connection(&db_path)
            .map_err(command_error)?
            .execute(
                "DELETE FROM provider_configs WHERE id = ?1",
                params![provider_id.to_string()],
            )
            .map_err(command_error)?;
        Ok(())
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn test_provider(
    state: State<'_, AppState>,
    input: ProviderIdInput,
) -> CommandResult<ProviderCapabilities> {
    let provider_id = parse_uuid(&input.provider_id, "providerId")?;
    let config = get_provider_config(&state.db_path, provider_id)?;
    let secret = read_provider_secret_with_store(&state.credential_store, &config)
        .map_err(command_error)?
        .ok_or_else(|| "provider credential was not found".to_owned())?;
    let provider = build_provider(
        &config,
        SecretString::new(secret),
        state.http_client.clone(),
    )
    .map_err(command_error)?;
    provider.test_connection().await.map_err(command_error)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchProviderModelsInput {
    kind: ProviderKind,
    base_url: Url,
    /// Key typed in the form; falls back to the stored secret of `provider_id`.
    api_key: Option<String>,
    provider_id: Option<String>,
}

#[tauri::command]
pub async fn fetch_provider_models(
    state: State<'_, AppState>,
    input: FetchProviderModelsInput,
) -> CommandResult<Vec<String>> {
    let secret = match input.api_key.filter(|key| !key.trim().is_empty()) {
        Some(key) => key,
        None => {
            let provider_id = input
                .provider_id
                .as_deref()
                .ok_or_else(|| "apiKey or providerId is required".to_owned())?;
            let provider_id = parse_uuid(provider_id, "providerId")?;
            let config = get_provider_config(&state.db_path, provider_id)?;
            read_provider_secret_with_store(&state.credential_store, &config)
                .map_err(command_error)?
                .ok_or_else(|| "provider credential was not found".to_owned())?
        }
    };

    list_models(
        input.kind,
        &input.base_url,
        &SecretString::new(secret),
        &state.http_client,
    )
    .await
    .map_err(command_error)
}

pub fn get_provider_config(
    db_path: &std::path::Path,
    provider_id: Uuid,
) -> CommandResult<ProviderConfig> {
    let connection = open_connection(db_path).map_err(command_error)?;
    connection
        .query_row(
            "SELECT id, name, kind, base_url, model, credential_ref FROM provider_configs WHERE id = ?1",
            params![provider_id.to_string()],
            provider_config_from_row,
        )
        .map_err(command_error)
}

pub fn list_provider_configs(db_path: &std::path::Path) -> CommandResult<Vec<ProviderConfig>> {
    let connection = open_connection(db_path).map_err(command_error)?;
    let mut statement = connection
        .prepare("SELECT id, name, kind, base_url, model, credential_ref FROM provider_configs ORDER BY created_at ASC")
        .map_err(command_error)?;
    let configs = statement
        .query_map([], provider_config_from_row)
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(configs)
}

fn upsert_provider_config(db_path: &std::path::Path, config: &ProviderConfig) -> CommandResult<()> {
    let now = Utc::now().to_rfc3339();
    open_connection(db_path)
        .map_err(command_error)?
        .execute(
            "INSERT INTO provider_configs (id, name, kind, base_url, model, credential_ref, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               kind = excluded.kind,
               base_url = excluded.base_url,
               model = excluded.model,
               credential_ref = excluded.credential_ref,
               updated_at = excluded.updated_at",
            params![
                config.id.to_string(),
                config.name,
                provider_kind_to_str(config.kind),
                config.base_url.as_str(),
                config.model,
                config.credential_ref,
                now,
            ],
        )
        .map_err(command_error)?;
    Ok(())
}

fn provider_config_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderConfig> {
    let id: String = row.get("id")?;
    let kind: String = row.get("kind")?;
    let base_url: String = row.get("base_url")?;
    Ok(ProviderConfig {
        id: Uuid::parse_str(&id)
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?,
        name: row.get("name")?,
        kind: provider_kind_from_str(&kind)?,
        base_url: Url::parse(&base_url)
            .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))?,
        model: row.get("model")?,
        credential_ref: row.get("credential_ref")?,
    })
}

fn provider_kind_to_str(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::OpenAi => "open_ai",
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::Gemini => "gemini",
        ProviderKind::OpenAiCompatible => "open_ai_compatible",
        ProviderKind::AnthropicCompatible => "anthropic_compatible",
    }
}

fn provider_kind_from_str(value: &str) -> rusqlite::Result<ProviderKind> {
    match value {
        "open_ai" => Ok(ProviderKind::OpenAi),
        "anthropic" => Ok(ProviderKind::Anthropic),
        "gemini" => Ok(ProviderKind::Gemini),
        "open_ai_compatible" => Ok(ProviderKind::OpenAiCompatible),
        "anthropic_compatible" => Ok(ProviderKind::AnthropicCompatible),
        _ => Err(rusqlite::Error::InvalidParameterName(format!(
            "invalid provider kind: {value}"
        ))),
    }
}

#[allow(dead_code)]
fn _parse_datetime(value: String) -> CommandResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(command_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_provider_input_accepts_rust_snake_case_provider_kind() {
        let input = serde_json::from_value::<SaveProviderInput>(serde_json::json!({
            "name": "Local",
            "kind": "open_ai_compatible",
            "baseUrl": "http://localhost:11434/v1",
            "model": "vision",
            "apiKey": "secret"
        }))
        .unwrap();

        assert_eq!(input.kind, ProviderKind::OpenAiCompatible);
    }

    #[test]
    fn save_provider_input_accepts_anthropic_compatible_provider_kind() {
        let input = serde_json::from_value::<SaveProviderInput>(serde_json::json!({
            "name": "Claude-compatible",
            "kind": "anthropic_compatible",
            "baseUrl": "http://localhost:11434",
            "model": "vision",
            "apiKey": "secret"
        }))
        .unwrap();

        assert_eq!(input.kind, ProviderKind::AnthropicCompatible);
        assert_eq!(provider_kind_to_str(input.kind), "anthropic_compatible");
        assert_eq!(
            provider_kind_from_str("anthropic_compatible").unwrap(),
            ProviderKind::AnthropicCompatible
        );
    }

    #[test]
    fn save_provider_input_rejects_legacy_openai_compatible_spelling() {
        let error = serde_json::from_value::<SaveProviderInput>(serde_json::json!({
            "name": "Local",
            "kind": "openai_compatible",
            "baseUrl": "http://localhost:11434/v1",
            "model": "vision",
            "apiKey": "secret"
        }))
        .unwrap_err();

        assert!(error.to_string().contains("unknown variant"));
    }
}
