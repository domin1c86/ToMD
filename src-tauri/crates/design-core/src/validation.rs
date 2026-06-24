use std::collections::HashSet;

use thiserror::Error;
use uuid::Uuid;

use crate::{DesignSpec, EvidenceRegion};

#[derive(Debug, Clone, PartialEq, Error)]
#[error("design specification validation failed with {count} issue(s)", count = .issues.len())]
pub struct ValidationError {
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ValidationIssue {
    #[error("schema version {schema_version:?} is not supported")]
    UnsupportedSchemaVersion { schema_version: String },
    #[error("rule {rule_id} has a blank statement")]
    BlankRuleStatement { rule_id: Uuid },
    #[error("uncertainty {uncertainty_id} has a blank statement")]
    BlankUncertaintyStatement { uncertainty_id: Uuid },
    #[error("rule {rule_id} has invalid confidence {confidence}")]
    InvalidConfidence { rule_id: Uuid, confidence: f32 },
    #[error("ID {id} is used more than once")]
    DuplicateId { id: Uuid },
    #[error("{owner_id} references missing evidence {evidence_id}")]
    MissingEvidence { owner_id: Uuid, evidence_id: Uuid },
    #[error("evidence {evidence_id} has an invalid normalized region")]
    InvalidRegion { evidence_id: Uuid },
    #[error("rule {rule_id} contains excluded term {term:?}")]
    ExcludedTermInRule { rule_id: Uuid, term: String },
}

pub(crate) fn validate(spec: &DesignSpec) -> Result<(), ValidationError> {
    let mut issues = Vec::new();

    if !has_supported_schema_major(&spec.metadata.schema_version) {
        issues.push(ValidationIssue::UnsupportedSchemaVersion {
            schema_version: spec.metadata.schema_version.clone(),
        });
    }

    let evidence_ids = spec
        .evidence
        .iter()
        .map(|evidence| evidence.id)
        .collect::<HashSet<_>>();
    let mut seen_ids = HashSet::new();

    for rule in rules(spec) {
        record_id(rule.id, &mut seen_ids, &mut issues);

        if rule.statement.trim().is_empty() {
            issues.push(ValidationIssue::BlankRuleStatement { rule_id: rule.id });
        }
        if !rule.confidence.is_finite() || !(0.0..=1.0).contains(&rule.confidence) {
            issues.push(ValidationIssue::InvalidConfidence {
                rule_id: rule.id,
                confidence: rule.confidence,
            });
        }
        record_missing_evidence(rule.id, &rule.evidence_ids, &evidence_ids, &mut issues);
    }

    for evidence in &spec.evidence {
        record_id(evidence.id, &mut seen_ids, &mut issues);

        if evidence.region.is_some_and(|region| !valid_region(region)) {
            issues.push(ValidationIssue::InvalidRegion {
                evidence_id: evidence.id,
            });
        }
    }

    for uncertainty in &spec.uncertainties {
        record_id(uncertainty.id, &mut seen_ids, &mut issues);

        if uncertainty.statement.trim().is_empty() {
            issues.push(ValidationIssue::BlankUncertaintyStatement {
                uncertainty_id: uncertainty.id,
            });
        }
        record_missing_evidence(
            uncertainty.id,
            &uncertainty.evidence_ids,
            &evidence_ids,
            &mut issues,
        );
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ValidationError { issues })
    }
}

fn has_supported_schema_major(schema_version: &str) -> bool {
    schema_version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u64>().ok())
        == Some(1)
}

fn record_id(id: Uuid, seen_ids: &mut HashSet<Uuid>, issues: &mut Vec<ValidationIssue>) {
    if !seen_ids.insert(id) {
        issues.push(ValidationIssue::DuplicateId { id });
    }
}

fn record_missing_evidence(
    owner_id: Uuid,
    references: &[Uuid],
    evidence_ids: &HashSet<Uuid>,
    issues: &mut Vec<ValidationIssue>,
) {
    issues.extend(
        references
            .iter()
            .filter(|evidence_id| !evidence_ids.contains(evidence_id))
            .map(|evidence_id| ValidationIssue::MissingEvidence {
                owner_id,
                evidence_id: *evidence_id,
            }),
    );
}

fn valid_region(region: EvidenceRegion) -> bool {
    let values = [region.x, region.y, region.width, region.height];

    values
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        && region.x + region.width <= 1.0
        && region.y + region.height <= 1.0
}

fn rules(spec: &DesignSpec) -> impl Iterator<Item = &crate::Rule> {
    spec.intent
        .iter()
        .chain(&spec.tokens)
        .chain(&spec.layout)
        .chain(&spec.components)
        .chain(&spec.assets)
        .chain(&spec.motion)
        .chain(&spec.constraints)
}
