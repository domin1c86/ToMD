use design_core::{DesignSpec, Rule, RuleStatus};
use design_storage::open_connection;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
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
pub struct UpdateRuleInput {
    project_id: String,
    rule_id: String,
    statement: Option<String>,
    status: Option<RuleStatus>,
}

#[tauri::command]
pub async fn get_design_spec(
    state: State<'_, AppState>,
    input: ProjectIdInput,
) -> CommandResult<DesignSpec> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || load_draft_spec(&db_path, project_id))
        .await
        .map_err(command_error)?
}

#[tauri::command]
pub async fn update_rule(
    state: State<'_, AppState>,
    input: UpdateRuleInput,
) -> CommandResult<DesignSpec> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let rule_id = parse_uuid(&input.rule_id, "ruleId")?;
    let db_path = state.db_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let connection = open_connection(&db_path).map_err(command_error)?;
        let mut spec = load_draft_spec(&db_path, project_id)?;
        let updated = update_rule_in_spec(&mut spec, rule_id, input.statement, input.status);
        if !updated {
            return Err("rule was not found".to_owned());
        }
        spec.validate().map_err(command_error)?;
        connection
            .execute(
                "UPDATE design_spec_drafts SET spec_json = ?1, updated_at = ?2 WHERE project_id = ?3",
                params![
                    serde_json::to_string(&spec).map_err(command_error)?,
                    chrono::Utc::now().to_rfc3339(),
                    project_id.to_string(),
                ],
            )
            .map_err(command_error)?;
        Ok(spec)
    })
    .await
    .map_err(command_error)?
}

pub fn load_draft_spec(db_path: &std::path::Path, project_id: Uuid) -> CommandResult<DesignSpec> {
    let connection = open_connection(db_path).map_err(command_error)?;
    let draft_json = connection
        .query_row(
            "SELECT spec_json FROM design_spec_drafts WHERE project_id = ?1",
            params![project_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(command_error)?;

    match draft_json {
        Some(json) => serde_json::from_str(&json).map_err(command_error),
        None => Ok(DesignSpec::empty(project_id.to_string())),
    }
}

fn update_rule_in_spec(
    spec: &mut DesignSpec,
    rule_id: Uuid,
    statement: Option<String>,
    status: Option<RuleStatus>,
) -> bool {
    for rule in all_rules_mut(spec) {
        if rule.id == rule_id {
            if let Some(statement) = statement {
                rule.statement = statement;
            }
            if let Some(status) = status {
                rule.status = status;
            }
            return true;
        }
    }

    false
}

fn all_rules_mut(spec: &mut DesignSpec) -> impl Iterator<Item = &mut Rule> {
    spec.intent
        .iter_mut()
        .chain(&mut spec.tokens)
        .chain(&mut spec.layout)
        .chain(&mut spec.components)
        .chain(&mut spec.assets)
        .chain(&mut spec.motion)
        .chain(&mut spec.constraints)
}
