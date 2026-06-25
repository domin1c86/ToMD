use design_core::{compile_markdown, DesignSpec, ValidationIssue};
use serde_json::json;

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

#[test]
fn rejects_exportable_categories_containing_excluded_terms() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.tokens[0].category = "OriginalBrand colors".to_owned();

    let error = compile_markdown(&spec).unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::ExcludedTermInRule { term, .. } if term == "OriginalBrand"
        )
    }));
}

#[test]
fn rejects_exportable_values_containing_excluded_terms() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.tokens[0].value = Some(json!({
        "brand": "OriginalBrand",
        "role": "primary_action"
    }));

    let error = compile_markdown(&spec).unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::ExcludedTermInRule { term, .. } if term == "OriginalBrand"
        )
    }));
}

#[test]
fn rejects_statement_excluded_terms_after_prose_whitespace_normalization() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.metadata.excluded_terms = vec!["Original Brand".to_owned()];
    spec.intent[0].statement =
        "Do not copy Original\n\t   Brand visual motifs into the export.".to_owned();

    let error = compile_markdown(&spec).unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::ExcludedTermInRule { term, .. } if term == "Original Brand"
        )
    }));
}

#[test]
fn rejects_category_excluded_terms_after_prose_whitespace_normalization() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.metadata.excluded_terms = vec!["Original Brand".to_owned()];
    spec.tokens[0].category = "Original\n   Brand colors".to_owned();

    let error = compile_markdown(&spec).unwrap_err();

    assert!(error.issues.iter().any(|issue| {
        matches!(
            issue,
            ValidationIssue::ExcludedTermInRule { term, .. } if term == "Original Brand"
        )
    }));
}

#[test]
fn sanitizes_statement_newlines_that_would_create_extra_headings() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.intent[0].statement =
        "Keep the visual hierarchy stable.\n# Extra section\nMore unsafe text.".to_owned();

    let output = compile_markdown(&spec).unwrap();

    assert_eq!(count_top_level_headings(&output), 10);
    assert!(!output.contains("\n# Extra section"));
    assert!(output.contains("Keep the visual hierarchy stable. # Extra section More unsafe text."));
}

#[test]
fn sanitizes_checklist_newlines_that_would_create_extra_items() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.constraints[0].statement =
        "Keep primary actions clear.\n- [ ] Injected checklist item\n# Extra section".to_owned();

    let output = compile_markdown(&spec).unwrap();
    let checklist = output
        .split_once("# AI implementation checklist\n\n")
        .map(|(_, checklist)| checklist)
        .unwrap();

    assert_eq!(count_top_level_headings(&output), 10);
    assert_eq!(checklist.matches("\n- [ ] ").count(), 1);
    assert!(!output.contains("\n- [ ] Injected checklist item"));
    assert!(!output.contains("\n# Extra section"));
    assert!(checklist.contains(
        "- [ ] Keep primary actions clear. - \\[ \\] Injected checklist item # Extra section"
    ));
}

#[test]
fn preserves_serialized_token_values_inside_safe_code_spans() {
    let mut spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    spec.tokens[0].value = Some(json!({
        "role": "literal_token",
        "sample": "Use `inline` and ```fence``` markers",
        "spacing": "keep  two   spaces",
        "line": "first\nsecond"
    }));
    let serialized = serde_json::to_string(spec.tokens[0].value.as_ref().unwrap()).unwrap();

    let output = compile_markdown(&spec).unwrap();

    assert!(serialized.contains('`'));
    assert!(serialized.contains("  "));
    assert!(serialized.contains("\\n"));
    assert!(output.contains(&format!("Value: ````{serialized}````")));
    assert!(!output.contains("Use 'inline'"));
    assert!(!output.contains("keep two spaces"));
}

fn count_top_level_headings(markdown: &str) -> usize {
    markdown
        .lines()
        .filter(|line| line.starts_with("# "))
        .count()
}
