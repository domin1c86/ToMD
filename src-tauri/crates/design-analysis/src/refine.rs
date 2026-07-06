use design_core::{DesignSpec, Rule, RuleSource, RuleStatus};
use design_providers::{AnalysisRequest, MultimodalProvider};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::orchestrator::{extract_json, AnalysisError};
use crate::prompt::FIXED_ANALYSIS_INSTRUCTIONS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefineScope {
    AllRules,
    Rule(Uuid),
}

#[derive(Debug, Clone)]
pub struct RefineOutcome {
    pub spec: DesignSpec,
    pub affected_rule_ids: Vec<Uuid>,
}

/// A single changed rule in the model's response. Extra fields are ignored
/// so models that echo category or status back do not fail parsing.
#[derive(Debug, Deserialize)]
struct PatchedRule {
    id: Uuid,
    statement: String,
}

pub fn refine_prompt(instruction: &str, rules_json: &str) -> String {
    format!(
        "{FIXED_ANALYSIS_INSTRUCTIONS}\n\n\
         Rewrite the design rules below according to the user's instruction.\n\
         Return only a JSON array of the changed rules, each item shaped as\n\
         {{\"id\": \"<uuid>\", \"statement\": \"<new statement>\"}}.\n\
         Use only ids from the provided rules. Do not invent new rules and do not\n\
         include rules whose statement is unchanged.\n\n\
         User instruction:\n{instruction}\n\n\
         Rules:\n{rules_json}"
    )
}

/// Applies a natural-language instruction to the draft rules via the
/// configured provider. Changed rules come back as `Edited` and still need
/// human acceptance; nothing is finalized here.
pub async fn refine_spec<P: MultimodalProvider + ?Sized>(
    provider: &P,
    model: &str,
    mut spec: DesignSpec,
    instruction: &str,
    scope: RefineScope,
) -> Result<RefineOutcome, AnalysisError> {
    if instruction.trim().is_empty() {
        return Err(AnalysisError::Refine("instruction must not be empty".to_owned()));
    }

    let targets: Vec<&Rule> = all_rules(&spec)
        .filter(|rule| match scope {
            RefineScope::AllRules => true,
            RefineScope::Rule(id) => rule.id == id,
        })
        .collect();
    if targets.is_empty() {
        return Err(AnalysisError::Refine(
            "no rules match the requested scope".to_owned(),
        ));
    }
    let target_ids: Vec<Uuid> = targets.iter().map(|rule| rule.id).collect();

    let rules_json = Value::Array(
        targets
            .iter()
            .map(|rule| {
                json!({
                    "id": rule.id,
                    "category": rule.category,
                    "statement": rule.statement,
                })
            })
            .collect(),
    )
    .to_string();
    let prompt = refine_prompt(instruction, &rules_json);

    let response = provider
        .analyze(AnalysisRequest {
            model: model.to_owned(),
            prompt,
            json_schema: Value::Null,
            images: Vec::new(),
        })
        .await?;

    let body = extract_json(&response.body).ok_or(AnalysisError::InvalidJson)?;
    let patches: Vec<PatchedRule> = serde_json::from_str(body).map_err(|error| {
        if error.is_data() {
            AnalysisError::InvalidSpec
        } else {
            AnalysisError::InvalidJson
        }
    })?;
    if patches.is_empty() {
        return Err(AnalysisError::Refine(
            "the model returned no rule changes".to_owned(),
        ));
    }

    let mut affected_rule_ids = Vec::with_capacity(patches.len());
    for patch in patches {
        if !target_ids.contains(&patch.id) {
            return Err(AnalysisError::InvalidSpec);
        }
        let statement = patch.statement.trim();
        if statement.is_empty() {
            return Err(AnalysisError::InvalidSpec);
        }
        if affected_rule_ids.contains(&patch.id) {
            return Err(AnalysisError::InvalidSpec);
        }
        let rule = all_rules_mut(&mut spec)
            .find(|rule| rule.id == patch.id)
            .expect("patched id was validated against targets");
        rule.statement = statement.to_owned();
        rule.status = RuleStatus::Edited;
        rule.source = RuleSource::Model;
        affected_rule_ids.push(patch.id);
    }

    spec.validate().map_err(|_| AnalysisError::InvalidSpec)?;

    Ok(RefineOutcome {
        spec,
        affected_rule_ids,
    })
}

fn all_rules(spec: &DesignSpec) -> impl Iterator<Item = &Rule> {
    spec.intent
        .iter()
        .chain(&spec.tokens)
        .chain(&spec.layout)
        .chain(&spec.components)
        .chain(&spec.assets)
        .chain(&spec.motion)
        .chain(&spec.constraints)
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
