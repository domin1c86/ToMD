use chrono::{DateTime, Utc};
use design_core::Platform;
use design_storage::{Project, ProjectRepository};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

use super::{command_error, parse_uuid, CommandResult};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProjectsInput {
    #[serde(default)]
    include_archived: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectInput {
    name: String,
    platform: Platform,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameProjectInput {
    project_id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIdInput {
    project_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectView {
    id: String,
    name: String,
    platform: Platform,
    archived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, AppState>,
    input: ListProjectsInput,
) -> CommandResult<Vec<ProjectView>> {
    state
        .storage
        .projects()
        .list(input.include_archived)
        .await
        .map(|projects| projects.into_iter().map(ProjectView::from).collect())
        .map_err(command_error)
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> CommandResult<ProjectView> {
    state
        .storage
        .projects()
        .create(&input.name, input.platform)
        .await
        .map(ProjectView::from)
        .map_err(command_error)
}

#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    input: RenameProjectInput,
) -> CommandResult<ProjectView> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    state
        .storage
        .projects()
        .rename(project_id, &input.name)
        .await
        .map(ProjectView::from)
        .map_err(command_error)
}

#[tauri::command]
pub async fn archive_project(
    state: State<'_, AppState>,
    input: ProjectIdInput,
) -> CommandResult<()> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    state
        .storage
        .projects()
        .archive(project_id)
        .await
        .map_err(command_error)
}

#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    input: ProjectIdInput,
) -> CommandResult<()> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    state
        .storage
        .projects()
        .delete(project_id)
        .await
        .map_err(command_error)
}

impl From<Project> for ProjectView {
    fn from(project: Project) -> Self {
        Self {
            id: project.id.to_string(),
            name: project.name,
            platform: project.platform,
            archived_at: project.archived_at,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}
