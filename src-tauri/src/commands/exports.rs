use std::{fs, path::PathBuf};

use chrono::{DateTime, Utc};
use design_core::compile_markdown;
use design_storage::open_connection;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;

use super::{command_error, parse_uuid, CommandResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIdInput {
    project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ExportDesignMarkdownInput {
    project_id: String,
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
        export_design_markdown_blocking(db_path, app_data_dir, project_id)
    })
    .await
    .map_err(command_error)?
}

fn list_exports_blocking(
    db_path: PathBuf,
    project_id: Uuid,
) -> CommandResult<Vec<ExportVersionView>> {
    let connection = open_connection(&db_path).map_err(command_error)?;
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
) -> CommandResult<ExportVersionView> {
    let mut connection = open_connection(&db_path).map_err(command_error)?;
    let draft_json: String = connection
        .query_row(
            "SELECT spec_json FROM design_spec_drafts WHERE project_id = ?1",
            params![project_id.to_string()],
            |row| row.get(0),
        )
        .map_err(command_error)?;
    let spec: design_core::DesignSpec = serde_json::from_str(&draft_json).map_err(command_error)?;
    let markdown = compile_markdown(&spec).map_err(command_error)?;
    let spec_version_id = Uuid::new_v4();
    let provider_id = spec
        .metadata
        .provider_id
        .clone()
        .unwrap_or_else(|| Uuid::nil().to_string());
    let model = spec
        .metadata
        .model
        .clone()
        .unwrap_or_else(|| "manual-draft".to_owned());

    let export_id = Uuid::new_v4();
    let relative_path = format!("exports/{export_id}.md");
    let destination = app_data_dir
        .join("projects")
        .join(project_id.to_string())
        .join(&relative_path);
    let created_at = Utc::now();
    let transaction = connection.transaction().map_err(command_error)?;
    transaction
        .execute(
            "INSERT INTO design_spec_versions (id, project_id, spec_json, provider_id, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                spec_version_id.to_string(),
                project_id.to_string(),
                serde_json::to_string(&spec).map_err(command_error)?,
                provider_id,
                model,
                created_at.to_rfc3339(),
            ],
        )
        .map_err(command_error)?;
    transaction
        .execute(
            "INSERT INTO export_versions (id, project_id, spec_version_id, relative_path, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                export_id.to_string(),
                project_id.to_string(),
                spec_version_id.to_string(),
                relative_path,
                created_at.to_rfc3339(),
            ],
        )
        .map_err(command_error)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(command_error)?;
    }
    fs::write(&destination, markdown).map_err(command_error)?;
    if let Err(error) = transaction.commit() {
        let _ = fs::remove_file(&destination);
        return Err(command_error(error));
    }

    Ok(ExportVersionView {
        id: export_id.to_string(),
        project_id: project_id.to_string(),
        spec_version_id: spec_version_id.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use design_core::{Evidence, Platform, Rule, RuleKind, RuleScope, RuleSource, RuleStatus};
    use rusqlite::Connection;

    #[test]
    fn export_input_rejects_destination_path() {
        let error = serde_json::from_value::<ExportDesignMarkdownInput>(serde_json::json!({
            "projectId": Uuid::new_v4().to_string(),
            "destinationPath": "C:/outside.md"
        }))
        .unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn export_snapshots_current_draft_before_recording_history() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("design-storage.sqlite3");
        let connection = design_storage::open_connection(&db_path).unwrap();
        create_schema(&connection);
        let project_id = Uuid::new_v4();
        let base_version_id = Uuid::new_v4();
        let provider_id = Uuid::new_v4();
        let spec = exportable_spec(project_id, provider_id);
        let spec_json = serde_json::to_string(&spec).unwrap();
        connection
            .execute(
                "INSERT INTO projects (id, name, platform, created_at, updated_at)
                 VALUES (?1, 'Project', 'web', ?2, ?2)",
                params![project_id.to_string(), Utc::now().to_rfc3339()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO design_spec_versions (id, project_id, spec_json, provider_id, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'old-model', ?5)",
                params![
                    base_version_id.to_string(),
                    project_id.to_string(),
                    spec_json,
                    provider_id.to_string(),
                    Utc::now().to_rfc3339(),
                ],
            )
            .unwrap();
        let mut edited = spec;
        edited.tokens[0].statement = "Edited exported rule.".to_owned();
        connection
            .execute(
                "INSERT INTO design_spec_drafts (project_id, base_version_id, spec_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    project_id.to_string(),
                    base_version_id.to_string(),
                    serde_json::to_string(&edited).unwrap(),
                    Utc::now().to_rfc3339(),
                ],
            )
            .unwrap();

        let export =
            export_design_markdown_blocking(db_path.clone(), temp.path().to_path_buf(), project_id)
                .unwrap();

        assert_ne!(export.spec_version_id, base_version_id.to_string());
        let exported_spec_json: String = connection
            .query_row(
                "SELECT spec_json FROM design_spec_versions WHERE id = ?1",
                params![export.spec_version_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exported_spec_json.contains("Edited exported rule."));
        assert!(temp
            .path()
            .join("projects")
            .join(project_id.to_string())
            .join(export.relative_path)
            .exists());
    }

    fn create_schema(connection: &Connection) {
        connection
            .execute_batch(
                r#"
                CREATE TABLE projects (
                  id TEXT PRIMARY KEY,
                  name TEXT NOT NULL,
                  platform TEXT NOT NULL,
                  archived_at TEXT,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                CREATE TABLE design_spec_versions (
                  id TEXT PRIMARY KEY,
                  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                  spec_json TEXT NOT NULL,
                  provider_id TEXT NOT NULL,
                  model TEXT NOT NULL,
                  created_at TEXT NOT NULL
                );
                CREATE TABLE design_spec_drafts (
                  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                  base_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
                  spec_json TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                CREATE TABLE export_versions (
                  id TEXT PRIMARY KEY,
                  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                  spec_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
                  relative_path TEXT NOT NULL,
                  created_at TEXT NOT NULL
                );
                "#,
            )
            .unwrap();
    }

    fn exportable_spec(project_id: Uuid, provider_id: Uuid) -> design_core::DesignSpec {
        let evidence_id = Uuid::new_v4();
        let mut spec = design_core::DesignSpec::empty(project_id.to_string());
        spec.metadata.platform = Platform::Web;
        spec.metadata.provider_id = Some(provider_id.to_string());
        spec.metadata.model = Some("vision-model".to_owned());
        spec.evidence = vec![Evidence {
            id: evidence_id,
            screenshot_id: Uuid::new_v4(),
            region: None,
            description: "Visible evidence.".to_owned(),
        }];
        spec.tokens = vec![Rule {
            id: Uuid::new_v4(),
            category: "Color".to_owned(),
            statement: "Accepted exported rule.".to_owned(),
            kind: RuleKind::Pattern,
            scope: RuleScope::Global,
            value: None,
            evidence_ids: vec![evidence_id],
            confidence: 0.9,
            status: RuleStatus::Accepted,
            source: RuleSource::Model,
        }];
        spec
    }
}
