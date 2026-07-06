use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use design_analysis::{refine_spec, AnalysisError, RefineScope};
use design_core::{DesignSpec, Evidence, Platform, Rule, RuleKind, RuleScope, RuleSource, RuleStatus};
use design_providers::{
    AnalysisRequest, MultimodalProvider, ProviderCapabilities, ProviderError, RawModelResponse,
};
use uuid::Uuid;

const PROJECT_ID: Uuid = Uuid::from_u128(0x01010101_0101_0101_0101_010101010101);
const SCREENSHOT_ID: Uuid = Uuid::from_u128(0x02020202_0202_0202_0202_020202020202);
const EVIDENCE_ID: Uuid = Uuid::from_u128(0x03030303_0303_0303_0303_030303030303);
const COLOR_RULE_ID: Uuid = Uuid::from_u128(0x04040404_0404_0404_0404_040404040404);
const LAYOUT_RULE_ID: Uuid = Uuid::from_u128(0x05050505_0505_0505_0505_050505050505);

#[tokio::test]
async fn applies_patches_as_edited_rules() {
    let provider = FakeProvider::new(format!(
        r#"[{{"id":"{COLOR_RULE_ID}","statement":"Primary actions use a muted blue range."}}]"#
    ));

    let outcome = refine_spec(
        &provider,
        "vision-model",
        spec(),
        "Rewrite color rules as ranges.",
        RefineScope::AllRules,
    )
    .await
    .unwrap();

    assert_eq!(outcome.affected_rule_ids, vec![COLOR_RULE_ID]);
    let rule = &outcome.spec.tokens[0];
    assert_eq!(rule.statement, "Primary actions use a muted blue range.");
    assert_eq!(rule.status, RuleStatus::Edited);
    assert_eq!(rule.source, RuleSource::Model);
    // untouched rule stays untouched
    assert_eq!(outcome.spec.layout[0].status, RuleStatus::Pending);

    let request = &provider.requests()[0];
    assert!(request.images.is_empty());
    assert!(request.json_schema.is_null());
    assert!(request.prompt.contains("Rewrite color rules as ranges."));
    assert!(request.prompt.contains(&COLOR_RULE_ID.to_string()));
    assert!(request.prompt.contains("Do not copy or emit brand names"));
}

#[tokio::test]
async fn scope_limits_the_rules_sent_to_the_provider() {
    let provider = FakeProvider::new(format!(
        r#"[{{"id":"{LAYOUT_RULE_ID}","statement":"Use 16-24px gutters."}}]"#
    ));

    refine_spec(
        &provider,
        "vision-model",
        spec(),
        "Loosen the gutter rule.",
        RefineScope::Rule(LAYOUT_RULE_ID),
    )
    .await
    .unwrap();

    let prompt = &provider.requests()[0].prompt;
    assert!(prompt.contains(&LAYOUT_RULE_ID.to_string()));
    assert!(!prompt.contains(&COLOR_RULE_ID.to_string()));
}

#[tokio::test]
async fn rejects_patches_for_unknown_rule_ids() {
    let unknown = Uuid::from_u128(0x0F0F0F0F_0F0F_0F0F_0F0F_0F0F0F0F0F0F);
    let provider =
        FakeProvider::new(format!(r#"[{{"id":"{unknown}","statement":"Invented rule."}}]"#));

    let error = refine_spec(
        &provider,
        "vision-model",
        spec(),
        "Change things.",
        RefineScope::AllRules,
    )
    .await
    .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidSpec);
}

#[tokio::test]
async fn rejects_out_of_scope_patches() {
    let provider = FakeProvider::new(format!(
        r#"[{{"id":"{COLOR_RULE_ID}","statement":"Out of scope edit."}}]"#
    ));

    let error = refine_spec(
        &provider,
        "vision-model",
        spec(),
        "Only touch the layout rule.",
        RefineScope::Rule(LAYOUT_RULE_ID),
    )
    .await
    .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidSpec);
}

#[tokio::test]
async fn rejects_empty_change_sets_and_empty_statements() {
    let provider = FakeProvider::new("[]".to_owned());
    let error = refine_spec(&provider, "m", spec(), "Do nothing.", RefineScope::AllRules)
        .await
        .unwrap_err();
    assert!(matches!(error, AnalysisError::Refine(_)));

    let provider = FakeProvider::new(format!(r#"[{{"id":"{COLOR_RULE_ID}","statement":"  "}}]"#));
    let error = refine_spec(&provider, "m", spec(), "Blank it.", RefineScope::AllRules)
        .await
        .unwrap_err();
    assert_eq!(error, AnalysisError::InvalidSpec);
}

#[tokio::test]
async fn rejects_empty_instruction_and_unknown_scope_without_network() {
    let provider = FakeProvider::new("[]".to_owned());

    let error = refine_spec(&provider, "m", spec(), "   ", RefineScope::AllRules)
        .await
        .unwrap_err();
    assert!(matches!(error, AnalysisError::Refine(_)));

    let missing = Uuid::from_u128(0x0E0E0E0E_0E0E_0E0E_0E0E_0E0E0E0E0E0E);
    let error = refine_spec(&provider, "m", spec(), "Edit.", RefineScope::Rule(missing))
        .await
        .unwrap_err();
    assert!(matches!(error, AnalysisError::Refine(_)));

    assert!(provider.requests().is_empty());
}

#[tokio::test]
async fn strips_markdown_fences_from_the_response() {
    let provider = FakeProvider::new(format!(
        "```json\n[{{\"id\":\"{COLOR_RULE_ID}\",\"statement\":\"Fenced edit.\"}}]\n```"
    ));

    let outcome = refine_spec(&provider, "m", spec(), "Edit.", RefineScope::AllRules)
        .await
        .unwrap();

    assert_eq!(outcome.spec.tokens[0].statement, "Fenced edit.");
}

fn spec() -> DesignSpec {
    let mut spec = DesignSpec::empty(PROJECT_ID.to_string());
    spec.metadata.platform = Platform::Web;
    spec.metadata.source_screenshot_ids = vec![SCREENSHOT_ID];
    spec.evidence = vec![Evidence {
        id: EVIDENCE_ID,
        screenshot_id: SCREENSHOT_ID,
        region: None,
        description: "Visible primary button and card layout".to_owned(),
    }];
    spec.tokens = vec![rule(COLOR_RULE_ID, "Color", "Primary actions use a vivid blue accent.")];
    spec.layout = vec![rule(LAYOUT_RULE_ID, "Spacing", "Pages use a 20px gutter.")];
    spec
}

fn rule(id: Uuid, category: &str, statement: &str) -> Rule {
    Rule {
        id,
        category: category.to_owned(),
        statement: statement.to_owned(),
        kind: RuleKind::Pattern,
        scope: RuleScope::Global,
        value: None,
        evidence_ids: vec![EVIDENCE_ID],
        confidence: 0.8,
        status: RuleStatus::Pending,
        source: RuleSource::Model,
    }
}

struct FakeProvider {
    body: String,
    requests: Arc<Mutex<Vec<AnalysisRequest>>>,
}

impl FakeProvider {
    fn new(body: String) -> Self {
        Self {
            body,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<AnalysisRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl MultimodalProvider for FakeProvider {
    async fn test_connection(&self) -> Result<ProviderCapabilities, ProviderError> {
        Ok(ProviderCapabilities {
            image_input: true,
            structured_output: true,
        })
    }

    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError> {
        self.requests.lock().unwrap().push(request);
        Ok(RawModelResponse {
            body: self.body.clone(),
            status_code: 200,
            request_id: None,
        })
    }
}
