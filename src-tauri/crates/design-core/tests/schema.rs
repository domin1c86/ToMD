use design_core::{
    DesignSpec, Evidence, EvidenceRegion, Rule, RuleKind, RuleStatus, Uncertainty, ValidationIssue,
};
use uuid::Uuid;

#[test]
fn empty_spec_uses_schema_version_one() {
    let spec = DesignSpec::empty("project-1");

    assert_eq!(spec.metadata.schema_version, "1.0");
}

#[test]
fn confidence_must_be_between_zero_and_one() {
    let mut spec = DesignSpec::empty("project-1");
    spec.intent.push(Rule::new(
        "color",
        "Use the accent color only for interactive emphasis.",
        RuleKind::Pattern,
        1.2,
    ));

    assert!(spec.validate().is_err());
}

#[test]
fn rejected_rules_are_not_exportable() {
    let mut rule = Rule::new(
        "layout",
        "Use a compact information density.",
        RuleKind::Recommendation,
        0.8,
    );
    rule.status = RuleStatus::Rejected;

    assert!(!rule.is_exportable());
}

#[test]
fn blank_rule_statements_are_rejected() {
    let mut spec = DesignSpec::empty("project-1");
    spec.intent
        .push(Rule::new("color", " \n\t", RuleKind::Observation, 0.7));

    let error = spec.validate().unwrap_err();

    assert!(error
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::BlankRuleStatement { .. })));
}

#[test]
fn blank_uncertainty_statements_are_rejected() {
    let mut spec = DesignSpec::empty("project-1");
    spec.uncertainties.push(Uncertainty {
        id: Uuid::new_v4(),
        statement: "   ".to_owned(),
        evidence_ids: Vec::new(),
    });

    let error = spec.validate().unwrap_err();

    assert!(error
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::BlankUncertaintyStatement { .. })));
}

#[test]
fn duplicate_ids_across_entity_types_are_rejected() {
    let duplicate_id = Uuid::new_v4();
    let mut rule = Rule::new("color", "Prefer blue.", RuleKind::Observation, 0.7);
    rule.id = duplicate_id;
    let mut spec = DesignSpec::empty("project-1");
    spec.intent.push(rule);
    spec.evidence.push(Evidence {
        id: duplicate_id,
        screenshot_id: Uuid::new_v4(),
        region: None,
        description: "Blue control".to_owned(),
    });

    let error = spec.validate().unwrap_err();

    assert!(error
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::DuplicateId { id } if *id == duplicate_id)));
}

#[test]
fn missing_evidence_references_are_rejected() {
    let missing_id = Uuid::new_v4();
    let mut rule = Rule::new("color", "Prefer blue.", RuleKind::Pattern, 0.7);
    rule.evidence_ids.push(missing_id);
    let mut spec = DesignSpec::empty("project-1");
    spec.intent.push(rule);

    let error = spec.validate().unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::MissingEvidence { evidence_id, .. } if *evidence_id == missing_id
        )
    }));
}

#[test]
fn unsupported_schema_major_is_rejected() {
    let mut spec = DesignSpec::empty("project-1");
    spec.metadata.schema_version = "2.0".to_owned();

    let error = spec.validate().unwrap_err();

    assert!(error
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::UnsupportedSchemaVersion { .. })));
}

#[test]
fn invalid_evidence_regions_are_rejected() {
    let evidence_id = Uuid::new_v4();
    let mut spec = DesignSpec::empty("project-1");
    spec.evidence.push(Evidence {
        id: evidence_id,
        screenshot_id: Uuid::new_v4(),
        region: Some(EvidenceRegion {
            x: 0.8,
            y: 0.0,
            width: 0.3,
            height: f32::INFINITY,
        }),
        description: "Invalid crop".to_owned(),
    });

    let error = spec.validate().unwrap_err();

    assert!(error.issues.iter().any(
        |issue| matches!(issue, ValidationIssue::InvalidRegion { evidence_id: id } if *id == evidence_id)
    ));
}

#[test]
fn nan_confidence_is_rejected() {
    let mut spec = DesignSpec::empty("project-1");
    spec.intent.push(Rule::new(
        "color",
        "Prefer blue.",
        RuleKind::Observation,
        f32::NAN,
    ));

    let error = spec.validate().unwrap_err();

    assert!(error
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::InvalidConfidence { .. })));
}
