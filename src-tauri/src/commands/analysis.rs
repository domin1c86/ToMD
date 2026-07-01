use std::{fs, path::PathBuf};

use async_trait::async_trait;
use design_analysis::{
    AnalysisError, AnalysisOrchestrator, AnalysisProject, AnalysisRepository, AnalysisScreenshot,
    StoredSpecVersion,
};
use design_core::DesignSpec;
use design_providers::{
    build_provider, read_provider_secret_with_store, AnalysisRequest, MultimodalProvider,
    ProviderCapabilities, ProviderError, RawModelResponse, SecretString,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;

use super::{command_error, parse_uuid, providers::get_provider_config, CommandResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSelectionInput {
    project_id: String,
    provider_id: String,
    screenshot_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalysisPreview {
    provider_name: String,
    model: String,
    image_ids: Vec<String>,
    image_count: usize,
    estimated_encoded_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeProjectOutput {
    version_id: String,
    repair_attempted: bool,
    spec: DesignSpec,
}

#[tauri::command]
pub async fn preview_analysis_request(
    state: State<'_, AppState>,
    input: AnalysisSelectionInput,
) -> CommandResult<AnalysisPreview> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let provider_id = parse_uuid(&input.provider_id, "providerId")?;
    let screenshot_ids = input
        .screenshot_ids
        .iter()
        .map(|id| parse_uuid(id, "screenshotId"))
        .collect::<CommandResult<Vec<_>>>()?;
    let provider = get_provider_config(&state.db_path, provider_id)?;
    let db_path = state.db_path.clone();
    let app_data_dir = state.app_data_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let estimated_encoded_bytes =
            estimate_encoded_screenshot_bytes(db_path, app_data_dir, project_id, &screenshot_ids)?;
        Ok(AnalysisPreview {
            provider_name: provider.name,
            model: provider.model,
            image_ids: screenshot_ids.iter().map(Uuid::to_string).collect(),
            image_count: screenshot_ids.len(),
            estimated_encoded_bytes,
        })
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn analyze_project(
    state: State<'_, AppState>,
    input: AnalysisSelectionInput,
) -> CommandResult<AnalyzeProjectOutput> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let provider_id = parse_uuid(&input.provider_id, "providerId")?;
    let screenshot_ids = input
        .screenshot_ids
        .iter()
        .map(|id| parse_uuid(id, "screenshotId"))
        .collect::<CommandResult<Vec<_>>>()?;
    let provider_config = get_provider_config(&state.db_path, provider_id)?;
    let secret = read_provider_secret_with_store(&state.credential_store, &provider_config)
        .map_err(command_error)?
        .ok_or_else(|| "provider credential was not found".to_owned())?;
    let provider = BoxedProvider(
        build_provider(
            &provider_config,
            SecretString::new(secret),
            state.http_client.clone(),
        )
        .map_err(command_error)?,
    );
    let repository = DesktopAnalysisRepository {
        db_path: state.db_path.clone(),
        app_data_dir: state.app_data_dir.clone(),
    };
    let orchestrator =
        AnalysisOrchestrator::new(repository, provider, provider_id, provider_config.model);
    let outcome = orchestrator
        .analyze_project(project_id, screenshot_ids)
        .await
        .map_err(command_error)?;

    Ok(AnalyzeProjectOutput {
        version_id: outcome.version_id.to_string(),
        repair_attempted: outcome.repair_attempted,
        spec: outcome.spec,
    })
}

fn estimate_encoded_screenshot_bytes(
    db_path: PathBuf,
    app_data_dir: PathBuf,
    project_id: Uuid,
    screenshot_ids: &[Uuid],
) -> CommandResult<u64> {
    let connection = Connection::open(db_path).map_err(command_error)?;
    let mut total = 0_u64;

    for screenshot_id in screenshot_ids {
        let relative_path: String = connection
            .query_row(
                "SELECT relative_path FROM screenshots WHERE project_id = ?1 AND id = ?2",
                params![project_id.to_string(), screenshot_id.to_string()],
                |row| row.get(0),
            )
            .map_err(command_error)?;
        let path = app_data_dir
            .join("projects")
            .join(project_id.to_string())
            .join(relative_path);
        let byte_len = fs::metadata(path).map_err(command_error)?.len();
        total = total.saturating_add(byte_len.div_ceil(3) * 4);
    }

    Ok(total)
}

struct BoxedProvider(Box<dyn MultimodalProvider>);

#[async_trait]
impl MultimodalProvider for BoxedProvider {
    async fn test_connection(&self) -> Result<ProviderCapabilities, ProviderError> {
        self.0.test_connection().await
    }

    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError> {
        self.0.analyze(request).await
    }
}

#[derive(Clone)]
struct DesktopAnalysisRepository {
    db_path: PathBuf,
    app_data_dir: PathBuf,
}

#[async_trait]
impl AnalysisRepository for DesktopAnalysisRepository {
    async fn load_project(&self, project_id: Uuid) -> Result<AnalysisProject, AnalysisError> {
        let db_path = self.db_path.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let connection = Connection::open(db_path).map_err(repository_error)?;
            let platform: String = connection
                .query_row(
                    "SELECT platform FROM projects WHERE id = ?1",
                    params![project_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(repository_error)?
                .ok_or_else(|| AnalysisError::Repository("project was not found".to_owned()))?;

            Ok(AnalysisProject {
                id: project_id,
                platform: platform_from_str(&platform)?,
                target_product_type: "local product".to_owned(),
            })
        })
        .await
        .map_err(repository_error)?
    }

    async fn load_screenshots(
        &self,
        project_id: Uuid,
        screenshot_ids: &[Uuid],
    ) -> Result<Vec<AnalysisScreenshot>, AnalysisError> {
        let db_path = self.db_path.clone();
        let app_data_dir = self.app_data_dir.clone();
        let screenshot_ids = screenshot_ids.to_vec();
        tauri::async_runtime::spawn_blocking(move || {
            let connection = Connection::open(db_path).map_err(repository_error)?;
            let mut screenshots = Vec::with_capacity(screenshot_ids.len());

            for screenshot_id in screenshot_ids {
                let row = connection
                    .query_row(
                        "SELECT relative_path, media_type, page_name, scene
                         FROM screenshots
                         WHERE project_id = ?1 AND id = ?2",
                        params![project_id.to_string(), screenshot_id.to_string()],
                        |row| {
                            Ok((
                                row.get::<_, String>("relative_path")?,
                                row.get::<_, String>("media_type")?,
                                row.get::<_, String>("page_name")?,
                                row.get::<_, String>("scene")?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(repository_error)?
                    .ok_or_else(|| {
                        AnalysisError::Repository(format!(
                            "screenshot {screenshot_id} was not found"
                        ))
                    })?;
                let path = app_data_dir
                    .join("projects")
                    .join(project_id.to_string())
                    .join(row.0);
                let bytes = fs::read(path).map_err(repository_error)?;
                screenshots.push(AnalysisScreenshot {
                    id: screenshot_id,
                    media_type: row.1,
                    page_name: row.2,
                    scene: row.3,
                    bytes,
                });
            }

            Ok(screenshots)
        })
        .await
        .map_err(repository_error)?
    }

    async fn insert_version_and_replace_draft(
        &self,
        project_id: Uuid,
        spec: DesignSpec,
        provider_id: Uuid,
        model: &str,
    ) -> Result<StoredSpecVersion, AnalysisError> {
        let db_path = self.db_path.clone();
        let model = model.to_owned();
        tauri::async_runtime::spawn_blocking(move || {
            let mut connection = Connection::open(db_path).map_err(repository_error)?;
            let transaction = connection.transaction().map_err(repository_error)?;
            let version_id = Uuid::new_v4();
            let spec_json = serde_json::to_string(&spec).map_err(repository_error)?;
            let now = chrono::Utc::now().to_rfc3339();
            transaction
                .execute(
                    "INSERT INTO design_spec_versions (id, project_id, spec_json, provider_id, model, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        version_id.to_string(),
                        project_id.to_string(),
                        spec_json,
                        provider_id.to_string(),
                        model,
                        now,
                    ],
                )
                .map_err(repository_error)?;
            transaction
                .execute(
                    "INSERT INTO design_spec_drafts (project_id, base_version_id, spec_json, updated_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(project_id) DO UPDATE SET
                       base_version_id = excluded.base_version_id,
                       spec_json = excluded.spec_json,
                       updated_at = excluded.updated_at",
                    params![
                        project_id.to_string(),
                        version_id.to_string(),
                        serde_json::to_string(&spec).map_err(repository_error)?,
                        chrono::Utc::now().to_rfc3339(),
                    ],
                )
                .map_err(repository_error)?;
            transaction.commit().map_err(repository_error)?;

            Ok(StoredSpecVersion {
                id: version_id,
                spec,
                provider_id,
                model,
            })
        })
        .await
        .map_err(repository_error)?
    }
}

fn repository_error(error: impl std::fmt::Display) -> AnalysisError {
    AnalysisError::Repository(error.to_string())
}

fn platform_from_str(value: &str) -> Result<design_core::Platform, AnalysisError> {
    match value {
        "web" => Ok(design_core::Platform::Web),
        "desktop" => Ok(design_core::Platform::Desktop),
        "mobile" => Ok(design_core::Platform::Mobile),
        "cross_platform" => Ok(design_core::Platform::CrossPlatform),
        _ => Err(AnalysisError::Repository(format!(
            "invalid platform: {value}"
        ))),
    }
}
