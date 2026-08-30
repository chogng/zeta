use crate::CollaborationShape;
use crate::EvalMode;
use crate::EvalRisk;
use serde::Serialize;
use std::collections::BTreeMap;
use zeta_protocol::ContentDigest;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelUsageSummary;

pub const EVALUATION_RESULT_SCHEMA_VERSION: u32 = 2;
pub const EVALUATION_PROTOCOL_REVISION: &str = "multi-agent-evals-v2";

/// Independent final judgment for one evaluation run.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvalStatus {
    Passed,
    Failed,
    Indeterminate,
}

/// One bounded host observation used by the result calculation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalFact {
    pub passed: bool,
    pub observation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_digest: Option<ContentDigest>,
}

impl EvalFact {
    pub(crate) fn new(passed: bool, observation: impl Into<String>) -> Self {
        Self {
            passed,
            observation: observation.into(),
            evidence_digest: None,
        }
    }

    pub(crate) fn with_digest(mut self, digest: ContentDigest) -> Self {
        self.evidence_digest = Some(digest);
        self
    }
}

/// Exact model and harness identity for the system under test.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalSubject {
    pub mode: EvalMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    pub label: String,
    pub evaluation_protocol_revision: String,
}

/// Machine-readable result whose status is derived only from the contained host facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalResult {
    schema_version: u32,
    case_id: String,
    case_digest: ContentDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comparison_group: Option<String>,
    collaboration_shape: CollaborationShape,
    risk: EvalRisk,
    subject: EvalSubject,
    status: EvalStatus,
    facts: BTreeMap<String, EvalFact>,
    usage: ModelUsageSummary,
    tool_call_count: u64,
    elapsed_millis: u64,
}

impl EvalResult {
    pub fn status(&self) -> EvalStatus {
        self.status
    }

    pub fn facts(&self) -> &BTreeMap<String, EvalFact> {
        &self.facts
    }

    pub(crate) fn from_facts(
        case: &crate::EvalCase,
        subject: EvalSubject,
        facts: BTreeMap<String, EvalFact>,
        usage: ModelUsageSummary,
        tool_call_count: u64,
        elapsed_millis: u64,
    ) -> Result<Self, String> {
        let case_bytes = serde_json::to_vec(case).map_err(|error| error.to_string())?;
        let status = if facts.values().all(|fact| fact.passed) {
            EvalStatus::Passed
        } else {
            EvalStatus::Failed
        };
        Ok(Self {
            schema_version: EVALUATION_RESULT_SCHEMA_VERSION,
            case_id: case.id.clone(),
            case_digest: ContentDigest::sha256(&case_bytes),
            comparison_group: case.comparison_group.clone(),
            collaboration_shape: case.collaboration_shape,
            risk: case.risk,
            subject,
            status,
            facts,
            usage,
            tool_call_count,
            elapsed_millis,
        })
    }

    pub(crate) fn indeterminate(
        case: &crate::EvalCase,
        subject: EvalSubject,
        reason: impl Into<String>,
        elapsed_millis: u64,
    ) -> Result<Self, String> {
        let case_bytes = serde_json::to_vec(case).map_err(|error| error.to_string())?;
        Ok(Self {
            schema_version: EVALUATION_RESULT_SCHEMA_VERSION,
            case_id: case.id.clone(),
            case_digest: ContentDigest::sha256(&case_bytes),
            comparison_group: case.comparison_group.clone(),
            collaboration_shape: case.collaboration_shape,
            risk: case.risk,
            subject,
            status: EvalStatus::Indeterminate,
            facts: BTreeMap::from([("evaluation_completed".into(), EvalFact::new(false, reason))]),
            usage: ModelUsageSummary::default(),
            tool_call_count: 0,
            elapsed_millis,
        })
    }
}
