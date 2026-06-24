use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use crate::ValidationError;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct DesignSpec {
    pub metadata: Metadata,
    pub intent: Vec<Rule>,
    pub tokens: Vec<Rule>,
    pub layout: Vec<Rule>,
    pub components: Vec<Rule>,
    pub assets: Vec<Rule>,
    pub motion: Vec<Rule>,
    pub constraints: Vec<Rule>,
    pub evidence: Vec<Evidence>,
    pub uncertainties: Vec<Uncertainty>,
}

impl DesignSpec {
    pub fn empty(project_id: impl Into<String>) -> Self {
        Self {
            metadata: Metadata {
                schema_version: "1.0".to_owned(),
                project_id: project_id.into(),
                platform: Platform::CrossPlatform,
                provider_id: None,
                model: None,
                source_screenshot_ids: Vec::new(),
                excluded_terms: Vec::new(),
                created_at: Utc::now(),
            },
            intent: Vec::new(),
            tokens: Vec::new(),
            layout: Vec::new(),
            components: Vec::new(),
            assets: Vec::new(),
            motion: Vec::new(),
            constraints: Vec::new(),
            evidence: Vec::new(),
            uncertainties: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        crate::validation::validate(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct Metadata {
    pub schema_version: String,
    pub project_id: String,
    pub platform: Platform,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub source_screenshot_ids: Vec<Uuid>,
    pub excluded_terms: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct Rule {
    pub id: Uuid,
    pub category: String,
    pub statement: String,
    pub kind: RuleKind,
    pub scope: RuleScope,
    #[ts(type = "unknown | null")]
    pub value: Option<Value>,
    pub evidence_ids: Vec<Uuid>,
    pub confidence: f32,
    pub status: RuleStatus,
    pub source: RuleSource,
}

impl Rule {
    pub fn new(
        category: impl Into<String>,
        statement: impl Into<String>,
        kind: RuleKind,
        confidence: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            category: category.into(),
            statement: statement.into(),
            kind,
            scope: RuleScope::Global,
            value: None,
            evidence_ids: Vec::new(),
            confidence,
            status: RuleStatus::Pending,
            source: RuleSource::Model,
        }
    }

    pub fn is_exportable(&self) -> bool {
        matches!(self.status, RuleStatus::Accepted | RuleStatus::Edited)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RuleKind {
    Observation,
    Pattern,
    Recommendation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RuleStatus {
    Pending,
    Accepted,
    Edited,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Platform,
    PageType,
    Component,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum RuleSource {
    Model,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum Platform {
    Web,
    Desktop,
    Mobile,
    CrossPlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct Evidence {
    pub id: Uuid,
    pub screenshot_id: Uuid,
    pub region: Option<EvidenceRegion>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS)]
pub struct EvidenceRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct Uncertainty {
    pub id: Uuid,
    pub statement: String,
    pub evidence_ids: Vec<Uuid>,
}
