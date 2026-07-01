use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use design_core::compile_markdown;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;

use super::{command_error, parse_uuid, rules::load_draft_spec, CommandResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIdInput {
    project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDesignMarkdownInput {
    project_id: String,
    destination_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct ExportVersionView {
    id: String,
    project_id: String,
    spec_version_id: String,
    relative_path: String,
    created_at: DateTime<Utc>,
}

#[tauri::command]
pub async fn list_exports(
    state: State<'_, AppState>,
    input: ProjectIdInput,
) -> CommandResult<Vec<ExportVersionView>> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || list_exports_blocking(db_path, project_id))
        .await
        .map_err(command_error)?
}

#[tauri::command]
pub async fn export_design_markdown(
    state: State<'_, AppState>,
    input: ExportDesignMarkdownInput,
) -> CommandResult<ExportVersionView> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let db_path = state.db_path.clone();
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        export_design_markdown_blocking(db_path, app_data_dir, project_id, input.destination_path)
    })
    .await
    .map_err(command_error)?
}

fn list_exports_blocking(
    db_path: PathBuf,
    project_id: Uuid,
) -> CommandResult<Vec<ExportVersionView>> {
    let connection = Connection::open(db_path).map_err(command_error)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, spec_version_id, relative_path, created_at
             FROM export_versions
             WHERE project_id = ?1
             ORDER BY created_at DESC",
        )
        .map_err(command_error)?;
    let exports = statement
        .query_map(params![project_id.to_string()], export_from_row)
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(exports)
}

fn export_design_markdown_blocking(
    db_path: PathBuf,
    app_data_dir: PathBuf,
    project_id: Uuid,
    destination_path: Option<PathBuf>,
) -> CommandResult<ExportVersionView> {
    let connection = Connection::open(&db_path).map_err(command_error)?;
    let spec = load_draft_spec(&db_path, project_id)?;
    let markdown = compile_markdown(&spec).map_err(command_error)?;
    let spec_version_id: String = connection
        .query_row(
            "SELECT base_version_id FROM design_spec_drafts WHERE project_id = ?1",
            params![project_id.to_string()],
            |row| row.get(0),
        )
        .map_err(command_error)?;

    let export_id = Uuid::new_v4();
    let relative_path = format!("exports/{export_id}.md");
    let destination = destination_path.unwrap_or_else(|| {
        app_data_dir
            .join("projects")
            .join(project_id.to_string())
            .join(&relative_path)
    });
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(command_error)?;
    }
    fs::write(&destination, markdown).map_err(command_error)?;

    let created_at = Utc::now();
    connection
        .execute(
            "INSERT INTO export_versions (id, project_id, spec_version_id, relative_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                export_id.to_string(),
                project_id.to_string(),
                spec_version_id,
                relative_path,
                created_at.to_rfc3339(),
            ],
        )
        .map_err(command_error)?;

    Ok(ExportVersionView {
        id: export_id.to_string(),
        project_id: project_id.to_string(),
        spec_version_id,
        relative_path,
        created_at,
    })
}

fn export_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExportVersionView> {
    Ok(ExportVersionView {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        spec_version_id: row.get("spec_version_id")?,
        relative_path: row.get("relative_path")?,
        created_at: parse_datetime(row.get("created_at")?)?,
    })
}

fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))
}
