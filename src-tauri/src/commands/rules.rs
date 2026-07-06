use design_analysis::{refine_spec, RefineScope};
use design_core::{DesignSpec, Rule, RuleStatus};
use design_providers::{build_provider, read_provider_secret_with_store, SecretString};
use design_storage::open_connection;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;

use super::{command_error, parse_uuid, providers::get_provider_config, CommandResult};

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineRulesInput {
    project_id: String,
    provider_id: String,
    instruction: String,
    /// Restricts the instruction to one rule; omitted = all rules.
    rule_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RefineRulesOutput {
    spec: DesignSpec,
    affected_rule_ids: Vec<String>,
}

#[tauri::command]
pub async fn refine_rules(
    state: State<'_, AppState>,
    input: RefineRulesInput,
) -> CommandResult<RefineRulesOutput> {
    let project_id = parse_uuid(&input.project_id, "projectId")?;
    let provider_id = parse_uuid(&input.provider_id, "providerId")?;
    let scope = match &input.rule_id {
        Some(rule_id) => RefineScope::Rule(parse_uuid(rule_id, "ruleId")?),
        None => RefineScope::AllRules,
    };

    let provider_config = get_provider_config(&state.db_path, provider_id)?;
    let secret = read_provider_secret_with_store(&state.credential_store, &provider_config)
        .map_err(command_error)?
        .ok_or_else(|| "provider credential was not found".to_owned())?;
    let provider = build_provider(
        &provider_config,
        SecretString::new(secret),
        state.http_client.clone(),
    )
    .map_err(command_error)?;

    let db_path = state.db_path.clone();
    let spec = tauri::async_runtime::spawn_blocking({
        let db_path = db_path.clone();
        move || load_draft_spec(&db_path, project_id)
    })
    .await
    .map_err(command_error)??;

    let outcome = refine_spec(
        provider.as_ref(),
        &provider_config.model,
        spec,
        &input.instruction,
        scope,
    )
    .await
    .map_err(command_error)?;

    let spec = outcome.spec;
    let persisted_spec = tauri::async_runtime::spawn_blocking(move || {
        let connection = open_connection(&db_path).map_err(command_error)?;
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
        Ok::<DesignSpec, String>(spec)
    })
    .await
    .map_err(command_error)??;

    Ok(RefineRulesOutput {
        spec: persisted_spec,
        affected_rule_ids: outcome
            .affected_rule_ids
            .iter()
            .map(Uuid::to_string)
            .collect(),
    })
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
