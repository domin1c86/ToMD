use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use design_storage::{open_connection, Screenshot, ScreenshotRepository};
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
#[serde(rename_all = "camelCase")]
pub struct ImportScreenshotsInput {
    project_id: String,
    paths: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateScreenshotMetadataInput {
    project_id: String,
    screenshot_id: String,
    page_name: String,
    scene: String,
    sort_order: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveScreenshotInput {
    project_id: String,
    screenshot_id: String,
}

#[derive(Debug, Serialize)]
pub struct ScreenshotView {
    id: String,
    project_id: String,
    relative_path: String,
    absolute_path: String,
    sha256: String,
    media_type: String,
    width: u32,
    height: u32,
    page_name: String,
    scene: String,
    sort_order: i64,
    created_at: DateTime<Utc>,
}

/// Absolute location of a stored screenshot so the webview can render it
/// through the asset protocol. Never sent to providers.
fn absolute_screenshot_path(app_data_dir: &Path, project_id: &str, relative_path: &str) -> String {
    app_data_dir
        .join("projects")
        .join(project_id)
        .join(relative_path)
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
pub async fn list_screenshots(
    state: State<'_, AppState>,
    input: ProjectIdInput,
) -> CommandResult<Vec<ScreenshotView>> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let db_path = state.db_path.clone();
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        list_screenshots_blocking(db_path, app_data_dir, project_id)
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn import_screenshots(
    state: State<'_, AppState>,
    input: ImportScreenshotsInput,
) -> CommandResult<Vec<ScreenshotView>> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let mut imported = Vec::with_capacity(input.paths.len());

    for path in input.paths {
        let page_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Screenshot")
            .to_owned();
        let screenshot = state
            .storage
            .screenshots()
            .import_screenshot(project_id, &path, &page_name, "")
            .await
            .map_err(command_error)?;
        imported.push(screenshot_view(screenshot, &state.app_data_dir));
    }

    Ok(imported)
}

#[tauri::command]
pub async fn update_screenshot_metadata(
    state: State<'_, AppState>,
    input: UpdateScreenshotMetadataInput,
) -> CommandResult<ScreenshotView> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let screenshot_id = parse_uuid(&input.screenshot_id, "screenshotId")?;
    let db_path = state.db_path.clone();
    let app_data_dir = state.app_data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        update_screenshot_metadata_blocking(
            db_path,
            app_data_dir,
            project_id,
            screenshot_id,
            input.page_name,
            input.scene,
            input.sort_order,
        )
    })
    .await
    .map_err(command_error)?
}

#[tauri::command]
pub async fn remove_screenshot(
    state: State<'_, AppState>,
    input: RemoveScreenshotInput,
) -> CommandResult<()> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let screenshot_id = parse_uuid(&input.screenshot_id, "screenshotId")?;
    state
        .storage
        .screenshots()
        .remove_screenshot(project_id, screenshot_id)
        .await
        .map_err(command_error)
}

fn list_screenshots_blocking(
    db_path: PathBuf,
    app_data_dir: PathBuf,
    project_id: Uuid,
) -> CommandResult<Vec<ScreenshotView>> {
    let connection = open_connection(&db_path).map_err(command_error)?;
    let mut statement = connection
        .prepare(
            "SELECT id, project_id, relative_path, sha256, media_type, width, height, page_name, scene, sort_order, created_at
             FROM screenshots
             WHERE project_id = ?1
             ORDER BY sort_order ASC, created_at ASC",
        )
        .map_err(command_error)?;
    let screenshots = statement
        .query_map(params![project_id.to_string()], |row| {
            screenshot_from_row(row, &app_data_dir)
        })
        .map_err(command_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(command_error)?;
    Ok(screenshots)
}

fn update_screenshot_metadata_blocking(
    db_path: PathBuf,
    app_data_dir: PathBuf,
    project_id: Uuid,
    screenshot_id: Uuid,
    page_name: String,
    scene: String,
    sort_order: i64,
) -> CommandResult<ScreenshotView> {
    let connection = open_connection(&db_path).map_err(command_error)?;
    let changed = connection
        .execute(
            "UPDATE screenshots
             SET page_name = ?1, scene = ?2, sort_order = ?3
             WHERE project_id = ?4 AND id = ?5",
            params![
                page_name,
                scene,
                sort_order,
                project_id.to_string(),
                screenshot_id.to_string()
            ],
        )
        .map_err(command_error)?;
    if changed == 0 {
        return Err("screenshot was not found".to_owned());
    }

    get_screenshot(&connection, &app_data_dir, project_id, screenshot_id)
}

fn get_screenshot(
    connection: &rusqlite::Connection,
    app_data_dir: &Path,
    project_id: Uuid,
    screenshot_id: Uuid,
) -> CommandResult<ScreenshotView> {
    connection
        .query_row(
            "SELECT id, project_id, relative_path, sha256, media_type, width, height, page_name, scene, sort_order, created_at
             FROM screenshots
             WHERE project_id = ?1 AND id = ?2",
            params![project_id.to_string(), screenshot_id.to_string()],
            |row| screenshot_from_row(row, app_data_dir),
        )
        .map_err(command_error)
}

fn screenshot_from_row(
    row: &rusqlite::Row<'_>,
    app_data_dir: &Path,
) -> rusqlite::Result<ScreenshotView> {
    let project_id: String = row.get("project_id")?;
    let relative_path: String = row.get("relative_path")?;
    Ok(ScreenshotView {
        id: row.get("id")?,
        absolute_path: absolute_screenshot_path(app_data_dir, &project_id, &relative_path),
        project_id,
        relative_path,
        sha256: row.get("sha256")?,
        media_type: row.get("media_type")?,
        width: row.get("width")?,
        height: row.get("height")?,
        page_name: row.get("page_name")?,
        scene: row.get("scene")?,
        sort_order: row.get("sort_order")?,
        created_at: parse_datetime(row.get("created_at")?)?,
    })
}

fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| rusqlite::Error::InvalidParameterName(error.to_string()))
}

fn screenshot_view(screenshot: Screenshot, app_data_dir: &Path) -> ScreenshotView {
    let project_id = screenshot.project_id.to_string();
    ScreenshotView {
        id: screenshot.id.to_string(),
        absolute_path: absolute_screenshot_path(
            app_data_dir,
            &project_id,
            &screenshot.relative_path,
        ),
        project_id,
        relative_path: screenshot.relative_path,
        sha256: screenshot.sha256,
        media_type: screenshot.media_type,
        width: screenshot.width,
        height: screenshot.height,
        page_name: screenshot.page_name,
        scene: screenshot.scene,
        sort_order: screenshot.sort_order,
        created_at: screenshot.created_at,
    }
}
