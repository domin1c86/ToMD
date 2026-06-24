use design_core::{compile_markdown, DesignSpec, ValidationIssue};

#[test]
fn compiles_only_confirmed_rules_in_fixed_section_order() {
    let spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();

    let output = compile_markdown(&spec).unwrap();

    assert_eq!(output, include_str!("fixtures/accepted-design.md"));
    assert!(!output.contains("OriginalBrand"));
    assert!(!output.contains("rejected rule"));
}

#[test]
fn rejects_exportable_statements_containing_excluded_terms() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.intent[0].statement = "OriginalBrand inspired dashboards are not allowed.".to_owned();

    let error = compile_markdown(&spec).unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::ExcludedTermInRule { term, .. } if term == "OriginalBrand"
        )
    }));
}
