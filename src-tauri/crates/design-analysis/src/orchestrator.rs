use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashSet;

use design_core::{DesignSpec, Platform, Rule, RuleKind, RuleStatus};
use design_providers::{AnalysisImage, AnalysisRequest, MultimodalProvider, ProviderError};
use schemars::schema_for;
use thiserror::Error;
use uuid::Uuid;

use crate::{prompt::build_analysis_prompt, repair::repair_prompt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisProject {
    pub id: Uuid,
    pub platform: Platform,
    pub target_product_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisScreenshot {
    pub id: Uuid,
    pub page_name: String,
    pub scene: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StoredSpecVersion {
    pub id: Uuid,
    pub spec: DesignSpec,
    pub provider_id: Uuid,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct AnalysisOutcome {
    pub version_id: Uuid,
    pub spec: DesignSpec,
    pub repair_attempted: bool,
}

#[derive(Debug, Error, PartialEq)]
pub enum AnalysisError {
    #[error("repository operation failed: {0}")]
    Repository(String),
    #[error("provider operation failed")]
    Provider(#[from] ProviderError),
    #[error("model response was not valid JSON")]
    InvalidJson,
    #[error("model response failed schema validation")]
    InvalidSpec,
    #[error("rule refinement failed: {0}")]
    Refine(String),
}

#[async_trait]
pub trait AnalysisRepository: Send + Sync {
    async fn load_project(&self, project_id: Uuid) -> Result<AnalysisProject, AnalysisError>;
    async fn load_screenshots(
        &self,
        project_id: Uuid,
        screenshot_ids: &[Uuid],
    ) -> Result<Vec<AnalysisScreenshot>, AnalysisError>;
    async fn insert_version_and_replace_draft(
        &self,
        project_id: Uuid,
        spec: DesignSpec,
        provider_id: Uuid,
        model: &str,
    ) -> Result<StoredSpecVersion, AnalysisError>;
}

pub struct AnalysisOrchestrator<R, P> {
    repository: R,
    provider: P,
    provider_id: Uuid,
    model: String,
}

impl<R, P> AnalysisOrchestrator<R, P>
where
    R: AnalysisRepository,
    P: MultimodalProvider,
{
    pub fn new(repository: R, provider: P, provider_id: Uuid, model: impl Into<String>) -> Self {
        Self {
            repository,
            provider,
            provider_id,
            model: model.into(),
        }
    }

    pub async fn analyze_project(
        &self,
        project_id: Uuid,
        screenshot_ids: Vec<Uuid>,
    ) -> Result<AnalysisOutcome, AnalysisError> {
        let project = self.repository.load_project(project_id).await?;
        let screenshots = self
            .repository
            .load_screenshots(project_id, &screenshot_ids)
            .await?;
        let schema = serde_json::to_value(schema_for!(DesignSpec)).map_err(|_| {
            AnalysisError::Repository("failed to serialize design spec schema".to_owned())
        })?;
        let prompt = build_analysis_prompt(
            project.id,
            project.platform,
            &project.target_product_type,
            &screenshots,
            &schema,
        );
        let images = screenshots
            .iter()
            .map(|screenshot| AnalysisImage {
                media_type: screenshot.media_type.clone(),
                bytes: screenshot.bytes.clone(),
            })
            .collect::<Vec<_>>();

        let first_response = self
            .provider
            .analyze(AnalysisRequest {
                model: self.model.clone(),
                prompt,
                json_schema: schema.clone(),
                images: images.clone(),
            })
            .await?;

        match parse_validate_and_prepare(
            &first_response.body,
            &project,
            self.provider_id,
            &self.model,
            &screenshot_ids,
        ) {
            Ok(spec) => self.persist(project_id, spec, false).await,
            Err(first_error) => {
                let repair_response = self
                    .provider
                    .analyze(AnalysisRequest {
                        model: self.model.clone(),
                        prompt: repair_prompt(&first_response.body, &first_error.to_string(), &schema),
                        json_schema: schema.clone(),
                        images,
                    })
                    .await?;
                let spec = parse_validate_and_prepare(
                    &repair_response.body,
                    &project,
                    self.provider_id,
                    &self.model,
                    &screenshot_ids,
                )?;
                self.persist(project_id, spec, true).await
            }
        }
    }

    async fn persist(
        &self,
        project_id: Uuid,
        spec: DesignSpec,
        repair_attempted: bool,
    ) -> Result<AnalysisOutcome, AnalysisError> {
        let version = self
            .repository
            .insert_version_and_replace_draft(project_id, spec, self.provider_id, &self.model)
            .await?;

        Ok(AnalysisOutcome {
            version_id: version.id,
            spec: version.spec,
            repair_attempted,
        })
    }
}

fn _request_shape(_request: AnalysisRequest, _image: AnalysisImage) {}

fn parse_validate_and_prepare(
    body: &str,
    project: &AnalysisProject,
    provider_id: Uuid,
    model: &str,
    screenshot_ids: &[Uuid],
) -> Result<DesignSpec, AnalysisError> {
    let json = extract_json(body).ok_or(AnalysisError::InvalidJson)?;
    let mut spec: DesignSpec = serde_json::from_str(json).map_err(|error| {
        if error.is_data() {
            AnalysisError::InvalidSpec
        } else {
            AnalysisError::InvalidJson
        }
    })?;

    spec.metadata.project_id = project.id.to_string();
    spec.metadata.platform = project.platform;
    spec.metadata.provider_id = Some(provider_id.to_string());
    spec.metadata.model = Some(model.to_owned());
    spec.metadata.source_screenshot_ids = screenshot_ids.to_vec();
    spec.metadata.created_at = Utc::now();
    normalize_rule_statuses(&mut spec);

    spec.validate().map_err(|_| AnalysisError::InvalidSpec)?;
    validate_provenance(&spec, screenshot_ids)?;
    Ok(spec)
}

pub(crate) fn extract_json(body: &str) -> Option<&str> {
    let trimmed = body.trim();
    if !trimmed.starts_with("```") {
        return Some(trimmed);
    }

    let after_opening_line = trimmed.find('\n').map(|index| &trimmed[index + 1..])?;
    let closing_index = after_opening_line.rfind("```")?;
    Some(after_opening_line[..closing_index].trim())
}

fn normalize_rule_statuses(spec: &mut DesignSpec) {
    for rule in spec
        .intent
        .iter_mut()
        .chain(&mut spec.tokens)
        .chain(&mut spec.layout)
        .chain(&mut spec.components)
        .chain(&mut spec.assets)
        .chain(&mut spec.motion)
        .chain(&mut spec.constraints)
    {
        rule.status = RuleStatus::Pending;
    }
}

fn validate_provenance(spec: &DesignSpec, screenshot_ids: &[Uuid]) -> Result<(), AnalysisError> {
    let selected_screenshot_ids = screenshot_ids.iter().copied().collect::<HashSet<_>>();

    if spec
        .evidence
        .iter()
        .any(|evidence| !selected_screenshot_ids.contains(&evidence.screenshot_id))
    {
        return Err(AnalysisError::InvalidSpec);
    }

    if all_rules(spec).any(|rule| {
        matches!(rule.kind, RuleKind::Pattern | RuleKind::Recommendation)
            && rule.evidence_ids.is_empty()
    }) {
        return Err(AnalysisError::InvalidSpec);
    }

    Ok(())
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
