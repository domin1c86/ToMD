use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use design_analysis::{
    AnalysisError, AnalysisOrchestrator, AnalysisProject, AnalysisRepository, AnalysisScreenshot,
    StoredSpecVersion,
};
use design_core::{
    DesignSpec, Evidence, Platform, Rule, RuleKind, RuleScope, RuleSource, RuleStatus,
};
use design_providers::{
    AnalysisRequest, MultimodalProvider, ProviderCapabilities, ProviderError, RawModelResponse,
};
use uuid::Uuid;

const PROJECT_ID: Uuid = Uuid::from_u128(0xaaaaaaaa_aaaa_aaaa_aaaa_aaaaaaaaaaaa);
const PROVIDER_ID: Uuid = Uuid::from_u128(0xbbbbbbbb_bbbb_bbbb_bbbb_bbbbbbbbbbbb);
const SCREENSHOT_ID: Uuid = Uuid::from_u128(0xcccccccc_cccc_cccc_cccc_cccccccccccc);
const EVIDENCE_ID: Uuid = Uuid::from_u128(0xdddddddd_dddd_dddd_dddd_dddddddddddd);
const RULE_ID: Uuid = Uuid::from_u128(0xeeeeeeee_eeee_eeee_eeee_eeeeeeeeeeee);

#[tokio::test]
async fn valid_first_response_creates_one_persisted_spec_version_with_provenance() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![raw_response(valid_spec_json())]);
    let provider_handle = provider.clone();
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let outcome = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap();

    assert!(!outcome.repair_attempted);
    assert_eq!(repository.versions().len(), 1);
    assert_eq!(repository.draft_replacements(), 1);
    let stored = &repository.versions()[0];
    assert_eq!(stored.version.provider_id, PROVIDER_ID);
    assert_eq!(stored.version.model, "vision-model");
    assert_eq!(stored.project_id, PROJECT_ID);
    assert_eq!(
        stored.version.spec.metadata.provider_id.as_deref(),
        Some(PROVIDER_ID.to_string().as_str())
    );
    assert_eq!(
        stored.version.spec.metadata.model.as_deref(),
        Some("vision-model")
    );
    assert_eq!(
        stored.version.spec.metadata.source_screenshot_ids,
        vec![SCREENSHOT_ID]
    );
    assert_eq!(outcome.version_id, stored.version.id);

    let prompt = &provider_handle.requests()[0].prompt;
    assert!(prompt.contains("Analyze only visible design evidence."));
    assert!(prompt.contains("Do not copy or emit brand names"));
    assert!(prompt.contains("Return only JSON matching the supplied schema."));
    assert!(prompt.contains("SaaS dashboard"));
    assert!(prompt.contains(&SCREENSHOT_ID.to_string()));
    assert!(prompt.contains("Home"));
    assert!(prompt.contains("Logged-in dashboard"));
    assert!(!prompt.contains("D:\\"));
    assert!(!prompt.contains("/tmp/"));
}

#[tokio::test]
async fn invalid_json_triggers_exactly_one_repair_request_and_persists_repaired_spec() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![
        raw_response("not json"),
        raw_response(valid_spec_json()),
    ]);
    let provider_handle = provider.clone();
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let outcome = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap();

    assert!(outcome.repair_attempted);
    assert_eq!(provider_handle.requests().len(), 2);
    assert!(provider_handle.requests()[1].prompt.contains("Repair"));
    assert_eq!(repository.versions().len(), 1);
}

#[tokio::test]
async fn repair_request_restates_privacy_rules_without_echoing_invalid_output() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![
        raw_response("BrandName customer@example.com D:\\secret\\shot.png"),
        raw_response(valid_spec_json()),
    ]);
    let provider_handle = provider.clone();
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let outcome = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap();

    assert!(outcome.repair_attempted);
    let repair_prompt = &provider_handle.requests()[1].prompt;
    assert!(repair_prompt.contains("Do not copy or emit brand names"));
    assert!(repair_prompt.contains("customer data"));
    assert!(repair_prompt.contains("Return only JSON matching the supplied schema."));
    assert!(!repair_prompt.contains("BrandName"));
    assert!(!repair_prompt.contains("customer@example.com"));
    assert!(!repair_prompt.contains("D:\\secret"));
}

#[tokio::test]
async fn invalid_repaired_output_creates_no_formal_version() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![raw_response("not json"), raw_response("{")]);
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let error = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidJson);
    assert!(repository.versions().is_empty());
}

#[tokio::test]
async fn valid_json_with_invalid_design_spec_shape_is_invalid_spec_not_invalid_json() {
    let repository = FakeRepository::default();
    let invalid_shape = include_str!("fixtures/invalid-response.json");
    let provider = FakeProvider::new(vec![
        raw_response(invalid_shape),
        raw_response(invalid_shape),
    ]);
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let error = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidSpec);
    assert!(repository.versions().is_empty());
}

#[tokio::test]
async fn missing_evidence_ids_fail_validation_and_do_not_persist() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![
        raw_response(spec_json_with_missing_evidence()),
        raw_response(spec_json_with_missing_evidence()),
    ]);
    let provider_handle = provider.clone();
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let error = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidSpec);
    assert_eq!(provider_handle.requests().len(), 2);
    assert!(repository.versions().is_empty());
}

#[tokio::test]
async fn evidence_from_unselected_screenshot_fails_validation_and_does_not_persist() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![
        raw_response(spec_json_with_foreign_screenshot_evidence()),
        raw_response(spec_json_with_foreign_screenshot_evidence()),
    ]);
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let error = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap_err();

    assert_eq!(error, AnalysisError::InvalidSpec);
    assert!(repository.versions().is_empty());
}

#[tokio::test]
async fn low_confidence_model_findings_remain_pending_for_user_adjustment() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![raw_response(valid_spec_json())]);
    let orchestrator = AnalysisOrchestrator::new(repository, provider, PROVIDER_ID, "vision-model");

    let outcome = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap();

    assert_eq!(outcome.spec.tokens[0].confidence, 0.42);
    assert_eq!(outcome.spec.tokens[0].status, RuleStatus::Pending);
}

#[tokio::test]
async fn conflicting_model_findings_remain_pending_for_user_adjustment() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::new(vec![raw_response(conflicting_spec_json())]);
    let orchestrator = AnalysisOrchestrator::new(repository, provider, PROVIDER_ID, "vision-model");

    let outcome = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap();

    assert_eq!(outcome.spec.tokens.len(), 2);
    assert!(outcome
        .spec
        .tokens
        .iter()
        .all(|rule| rule.status == RuleStatus::Pending));
}

#[tokio::test]
async fn provider_failure_creates_no_version_and_preserves_project_state() {
    let repository = FakeRepository::default();
    let provider = FakeProvider::failing(ProviderError::RateLimited);
    let orchestrator =
        AnalysisOrchestrator::new(repository.clone(), provider, PROVIDER_ID, "vision-model");

    let error = orchestrator
        .analyze_project(PROJECT_ID, vec![SCREENSHOT_ID])
        .await
        .unwrap_err();

    assert_eq!(error, AnalysisError::Provider(ProviderError::RateLimited));
    assert!(repository.versions().is_empty());
}

#[derive(Clone, Default)]
struct FakeRepository {
    versions: Arc<Mutex<Vec<StoredVersionRecord>>>,
    draft_replacements: Arc<Mutex<usize>>,
}

impl FakeRepository {
    fn versions(&self) -> Vec<StoredVersionRecord> {
        self.versions.lock().unwrap().clone()
    }

    fn draft_replacements(&self) -> usize {
        *self.draft_replacements.lock().unwrap()
    }
}

#[derive(Debug, Clone)]
struct StoredVersionRecord {
    project_id: Uuid,
    version: StoredSpecVersion,
}

#[async_trait]
impl AnalysisRepository for FakeRepository {
    async fn load_project(&self, project_id: Uuid) -> Result<AnalysisProject, AnalysisError> {
        assert_eq!(project_id, PROJECT_ID);
        Ok(AnalysisProject {
            id: project_id,
            platform: Platform::Web,
            target_product_type: "SaaS dashboard".to_owned(),
        })
    }

    async fn load_screenshots(
        &self,
        project_id: Uuid,
        screenshot_ids: &[Uuid],
    ) -> Result<Vec<AnalysisScreenshot>, AnalysisError> {
        assert_eq!(project_id, PROJECT_ID);
        assert_eq!(screenshot_ids, &[SCREENSHOT_ID]);
        Ok(vec![AnalysisScreenshot {
            id: SCREENSHOT_ID,
            page_name: "Home".to_owned(),
            scene: "Logged-in dashboard".to_owned(),
            media_type: "image/png".to_owned(),
            bytes: vec![1, 2, 3],
        }])
    }

    async fn insert_version_and_replace_draft(
        &self,
        project_id: Uuid,
        spec: DesignSpec,
        provider_id: Uuid,
        model: &str,
    ) -> Result<StoredSpecVersion, AnalysisError> {
        let version = StoredSpecVersion {
            id: Uuid::from_u128(0xffffeeee_dddd_cccc_bbbb_aaaaaaaaaaaa),
            spec,
            provider_id,
            model: model.to_owned(),
        };
        self.versions.lock().unwrap().push(StoredVersionRecord {
            project_id,
            version: version.clone(),
        });
        *self.draft_replacements.lock().unwrap() += 1;
        Ok(version)
    }
}

#[derive(Clone)]
struct FakeProvider {
    responses: Arc<Mutex<Vec<Result<RawModelResponse, ProviderError>>>>,
    requests: Arc<Mutex<Vec<AnalysisRequest>>>,
}

impl FakeProvider {
    fn new(responses: Vec<RawModelResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().map(Ok).collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing(error: ProviderError) -> Self {
        Self {
            responses: Arc::new(Mutex::new(vec![Err(error)])),
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
        self.responses.lock().unwrap().remove(0)
    }
}

fn raw_response(body: impl Into<String>) -> RawModelResponse {
    RawModelResponse {
        body: body.into(),
        status_code: 200,
        request_id: Some("request-id".to_owned()),
    }
}

fn valid_spec_json() -> String {
    serde_json::to_string(&valid_spec()).unwrap()
}

fn spec_json_with_missing_evidence() -> String {
    let mut spec = valid_spec();
    spec.tokens[0].evidence_ids = vec![Uuid::from_u128(0x11111111_1111_1111_1111_111111111111)];
    serde_json::to_string(&spec).unwrap()
}

fn spec_json_with_foreign_screenshot_evidence() -> String {
    let mut spec = valid_spec();
    spec.evidence[0].screenshot_id = Uuid::from_u128(0x22222222_2222_2222_2222_222222222222);
    serde_json::to_string(&spec).unwrap()
}

fn conflicting_spec_json() -> String {
    let mut spec = valid_spec();
    let mut conflicting = spec.tokens[0].clone();
    conflicting.id = Uuid::from_u128(0x99999999_9999_9999_9999_999999999999);
    conflicting.statement = "Primary actions use a muted gray treatment.".to_owned();
    conflicting.confidence = 0.91;
    conflicting.status = RuleStatus::Accepted;
    spec.tokens.push(conflicting);
    serde_json::to_string(&spec).unwrap()
}

fn valid_spec() -> DesignSpec {
    let mut spec = DesignSpec::empty(PROJECT_ID.to_string());
    spec.metadata.platform = Platform::Web;
    spec.metadata.source_screenshot_ids = vec![SCREENSHOT_ID];
    spec.evidence = vec![Evidence {
        id: EVIDENCE_ID,
        screenshot_id: SCREENSHOT_ID,
        region: None,
        description: "Visible primary button and card layout".to_owned(),
    }];
    spec.tokens = vec![Rule {
        id: RULE_ID,
        category: "Color".to_owned(),
        statement: "Primary actions use a vivid blue accent.".to_owned(),
        kind: RuleKind::Pattern,
        scope: RuleScope::Global,
        value: None,
        evidence_ids: vec![EVIDENCE_ID],
        confidence: 0.42,
        status: RuleStatus::Accepted,
        source: RuleSource::Model,
    }];
    spec
}
