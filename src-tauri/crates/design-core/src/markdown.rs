use std::cmp::Ordering;

use serde_json::Value;

use crate::{DesignSpec, Rule, RuleKind, ValidationError, ValidationIssue};

const DESIGN_INTENT: &str = "Design intent";
const VISUAL_PRINCIPLES: &str = "Visual principles";
const DESIGN_TOKENS: &str = "Design tokens";
const TYPOGRAPHY: &str = "Typography";
const LAYOUT: &str = "Layout and responsive rules";
const COMPONENTS: &str = "Component conventions";
const INTERACTION: &str = "Interaction and states";
const ASSETS: &str = "Image and icon direction";
const DO_DONT: &str = "Do / Don't";
const CHECKLIST: &str = "AI implementation checklist";

pub fn compile_markdown(spec: &DesignSpec) -> Result<String, ValidationError> {
    spec.validate()?;
    reject_excluded_terms(spec)?;

    let sections = [
        (
            DESIGN_INTENT,
            exportable_rules(&spec.intent)
                .into_iter()
                .filter(|rule| !is_visual_principle(rule))
                .collect::<Vec<_>>(),
        ),
        (
            VISUAL_PRINCIPLES,
            exportable_rules(&spec.intent)
                .into_iter()
                .filter(|rule| is_visual_principle(rule))
                .collect::<Vec<_>>(),
        ),
        (
            DESIGN_TOKENS,
            exportable_rules(&spec.tokens)
                .into_iter()
                .filter(|rule| !is_typography(rule))
                .collect::<Vec<_>>(),
        ),
        (
            TYPOGRAPHY,
            exportable_rules(&spec.tokens)
                .into_iter()
                .filter(|rule| is_typography(rule))
                .collect::<Vec<_>>(),
        ),
        (LAYOUT, exportable_rules(&spec.layout)),
        (COMPONENTS, exportable_rules(&spec.components)),
        (INTERACTION, exportable_rules(&spec.motion)),
        (ASSETS, exportable_rules(&spec.assets)),
        (DO_DONT, exportable_rules(&spec.constraints)),
    ];

    let mut output = String::new();
    for (title, mut rules) in sections {
        sort_rules(&mut rules);
        push_rule_section(&mut output, title, &rules);
    }

    let mut constraints = exportable_rules(&spec.constraints);
    sort_rules(&mut constraints);
    push_checklist_section(&mut output, &constraints);

    Ok(output)
}

fn reject_excluded_terms(spec: &DesignSpec) -> Result<(), ValidationError> {
    let excluded_terms = spec
        .metadata
        .excluded_terms
        .iter()
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if excluded_terms.is_empty() {
        return Ok(());
    }

    let issues = all_rules(spec)
        .filter(|rule| rule.is_exportable())
        .flat_map(|rule| {
            excluded_terms
                .iter()
                .filter(move |term| rule.statement.contains(term.as_str()))
                .map(move |term| ValidationIssue::ExcludedTermInRule {
                    rule_id: rule.id,
                    term: (*term).clone(),
                })
        })
        .collect::<Vec<_>>();

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ValidationError { issues })
    }
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

fn exportable_rules(rules: &[Rule]) -> Vec<&Rule> {
    rules.iter().filter(|rule| rule.is_exportable()).collect()
}

fn sort_rules(rules: &mut [&Rule]) {
    rules.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| compare_confidence_desc(left.confidence, right.confidence))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn compare_confidence_desc(left: f32, right: f32) -> Ordering {
    right
        .partial_cmp(&left)
        .expect("DesignSpec validation rejects non-finite confidence")
}

fn push_rule_section(output: &mut String, title: &str, rules: &[&Rule]) {
    output.push_str("# ");
    output.push_str(title);
    output.push_str("\n\n");

    for rule in rules {
        output.push_str("- **");
        output.push_str(&render_markdown_text(&rule.category));
        output.push_str("** (");
        output.push_str(rule_kind_label(rule.kind));
        output.push_str(", confidence ");
        output.push_str(&format!("{:.2}", rule.confidence));
        output.push_str("): ");
        output.push_str(&render_markdown_text(&rule.statement));
        if let Some(value) = &rule.value {
            output.push_str(" Value: `");
            output.push_str(&render_inline_code_text(&render_value(value)));
            output.push('`');
        }
        output.push('\n');
    }

    output.push('\n');
}

fn push_checklist_section(output: &mut String, constraints: &[&Rule]) {
    output.push_str("# ");
    output.push_str(CHECKLIST);
    output.push_str("\n\n");

    for constraint in constraints {
        output.push_str("- [ ] ");
        output.push_str(&render_markdown_text(&constraint.statement));
        output.push('\n');
    }
}

fn is_visual_principle(rule: &Rule) -> bool {
    let category = rule.category.to_ascii_lowercase();
    category.contains("visual") || category.contains("principle")
}

fn is_typography(rule: &Rule) -> bool {
    let category = rule.category.to_ascii_lowercase();
    category.contains("typography")
        || category.contains("type")
        || category.contains("font")
        || category.contains("text")
}

fn rule_kind_label(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Observation => "observation",
        RuleKind::Pattern => "pattern",
        RuleKind::Recommendation => "recommendation",
    }
}

fn render_value(value: &Value) -> String {
    serde_json::to_string(value).expect("serializing serde_json::Value cannot fail")
}

fn render_markdown_text(input: &str) -> String {
    let normalized = normalize_control_whitespace(input);
    let mut escaped = String::with_capacity(normalized.len());

    for character in normalized.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }

    escaped
}

fn render_inline_code_text(input: &str) -> String {
    normalize_control_whitespace(input).replace('`', "'")
}

fn normalize_control_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
