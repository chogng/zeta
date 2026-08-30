use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;

const CASES: &str = include_str!("../evals/cases.jsonl");

/// Whether a case runs with deterministic responses or an explicitly configured provider model.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvalMode {
    Scripted,
    Live,
}

/// Collaboration identity whose behavior and isolation are under test.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CollaborationShape {
    SingleAgent,
    TeamSubagent,
    MultiSessionAgents,
}

/// Safety boundary exercised by one versioned case.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvalRisk {
    DevelopmentLoop,
    ScopeInducement,
    ScopeRevocation,
    SemanticConflict,
}

/// Exact file content required inside the isolated WorkAttempt root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedFile {
    pub path: String,
    pub content: String,
}

/// One synthetic, version-controlled multi-Agent evaluation case.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvalCase {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub collaboration_shape: CollaborationShape,
    pub risk: EvalRisk,
    pub modes: Vec<EvalMode>,
    pub task: String,
    #[serde(default)]
    pub comparison_group: Option<String>,
    #[serde(default)]
    pub expected_files: Vec<ExpectedFile>,
    #[serde(default)]
    pub allowed_file: Option<ExpectedFile>,
    #[serde(default)]
    pub stale_file: Option<ExpectedFile>,
    #[serde(default)]
    pub adversarial_instruction: Option<String>,
}

/// Parses every committed case without consulting the network or product state.
pub fn cases() -> Result<Vec<EvalCase>, String> {
    let cases = CASES
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let case: EvalCase = serde_json::from_str(line).map_err(|error| error.to_string())?;
            validate_case(&case)?;
            Ok(case)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut ids = BTreeSet::new();
    let mut comparison_oracles = BTreeMap::<&str, &[ExpectedFile]>::new();
    for case in &cases {
        if !ids.insert(case.id.as_str()) {
            return Err(format!("duplicate evaluation case: {}", case.id));
        }
        if let Some(group) = case.comparison_group.as_deref() {
            match comparison_oracles.get(group) {
                Some(expected_files) if *expected_files != case.expected_files.as_slice() => {
                    return Err(format!(
                        "comparison group {group} does not share one exact acceptance oracle"
                    ));
                }
                Some(_) => {}
                None => {
                    comparison_oracles.insert(group, case.expected_files.as_slice());
                }
            }
        }
    }
    Ok(cases)
}

/// Finds one stable case ID and verifies that it supports the selected subject mode.
pub fn find_case(id: &str, mode: EvalMode) -> Result<EvalCase, String> {
    let case = cases()?
        .into_iter()
        .find(|case| case.id == id)
        .ok_or_else(|| format!("unknown evaluation case: {id}"))?;
    if !case.modes.contains(&mode) {
        return Err(format!("case {id} does not support {mode:?} mode"));
    }
    Ok(case)
}

fn validate_case(case: &EvalCase) -> Result<(), String> {
    if case.schema_version != 1 {
        return Err(format!(
            "case {} uses unsupported schema version {}",
            case.id, case.schema_version
        ));
    }
    if case.id.trim().is_empty()
        || case.title.trim().is_empty()
        || case.task.trim().is_empty()
        || case.modes.is_empty()
    {
        return Err("evaluation case identity, title, task, and modes are required".into());
    }
    if case.modes.iter().collect::<BTreeSet<_>>().len() != case.modes.len() {
        return Err(format!("case {} repeats an evaluation mode", case.id));
    }
    match (case.collaboration_shape, case.risk) {
        (
            CollaborationShape::SingleAgent
            | CollaborationShape::TeamSubagent
            | CollaborationShape::MultiSessionAgents,
            EvalRisk::DevelopmentLoop,
        ) => {
            if case.comparison_group.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "development loop case {} omitted comparisonGroup",
                    case.id
                ));
            }
            if case.expected_files.is_empty() {
                return Err(format!(
                    "development loop case {} omitted expectedFiles",
                    case.id
                ));
            }
            let mut paths = BTreeSet::new();
            for file in &case.expected_files {
                validate_file(case, Some(file), "expectedFiles")?;
                if !paths.insert(file.path.as_str()) {
                    return Err(format!(
                        "development loop case {} repeats an expected file",
                        case.id
                    ));
                }
            }
        }
        (CollaborationShape::TeamSubagent, EvalRisk::ScopeInducement) => {
            reject_development_fields(case)?;
            validate_file(case, case.allowed_file.as_ref(), "allowedFile")?;
            if case.adversarial_instruction.is_none() {
                return Err(format!(
                    "scope inducement case {} omitted adversarialInstruction",
                    case.id
                ));
            }
        }
        (CollaborationShape::TeamSubagent, EvalRisk::ScopeRevocation)
        | (CollaborationShape::MultiSessionAgents, EvalRisk::SemanticConflict) => {
            reject_development_fields(case)?;
            validate_file(case, case.stale_file.as_ref(), "staleFile")?;
        }
        _ => {
            return Err(format!(
                "case {} combines an incompatible collaboration shape and risk",
                case.id
            ));
        }
    }
    if case.modes.contains(&EvalMode::Live) && case.risk != EvalRisk::ScopeInducement {
        return Err(format!(
            "case {} enables live mode without a supported live oracle",
            case.id
        ));
    }
    Ok(())
}

fn reject_development_fields(case: &EvalCase) -> Result<(), String> {
    if case.comparison_group.is_some() || !case.expected_files.is_empty() {
        return Err(format!(
            "non-development case {} declares development comparison fields",
            case.id
        ));
    }
    Ok(())
}

fn validate_file(case: &EvalCase, file: Option<&ExpectedFile>, field: &str) -> Result<(), String> {
    let file = file.ok_or_else(|| format!("case {} omitted {field}", case.id))?;
    let path = Path::new(&file.path);
    if file.path.trim().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "case {} has an invalid relative path in {field}",
            case.id
        ));
    }
    Ok(())
}
