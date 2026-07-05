use design_core::Platform;
use serde_json::Value;
use uuid::Uuid;

use crate::orchestrator::AnalysisScreenshot;

pub const FIXED_ANALYSIS_INSTRUCTIONS: &str = r#"Analyze only visible design evidence.
Separate observations, cross-screenshot patterns, and recommendations.
Do not copy or emit brand names, logos, original product copy, customer data, or proprietary assets.
Every pattern and recommendation must cite evidence IDs.
Use exact values only when visually supported; otherwise describe a range or principle.
Return only JSON matching the supplied schema."#;

pub fn build_analysis_prompt(
    project_id: Uuid,
    platform: Platform,
    target_product_type: &str,
    screenshots: &[AnalysisScreenshot],
    schema: &Value,
) -> String {
    let mut prompt = String::from(FIXED_ANALYSIS_INSTRUCTIONS);
    prompt.push_str("\n\nProject context:\n");
    prompt.push_str(&format!("- project_id: {project_id}\n"));
    prompt.push_str(&format!("- platform: {platform:?}\n"));
    prompt.push_str(&format!("- target_product_type: {target_product_type}\n"));
    prompt.push_str("- screenshots:\n");

    for screenshot in screenshots {
        prompt.push_str(&format!(
            "  - id: {}; page_name: {}; scene: {}\n",
            screenshot.id, screenshot.page_name, screenshot.scene
        ));
    }

    push_schema_section(&mut prompt, schema);
    prompt
}

/// Providers request plain JSON output, so the schema travels inside the
/// prompt instead of the request's structured-output constraint.
pub(crate) fn push_schema_section(prompt: &mut String, schema: &Value) {
    if schema.is_null() {
        return;
    }
    prompt.push_str("\nThe supplied schema:\n");
    prompt.push_str(&schema.to_string());
    prompt.push('\n');
}
