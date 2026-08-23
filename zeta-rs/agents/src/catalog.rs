use crate::model::AgentDefinition;
use crate::model::AgentDefinitionCatalogSnapshot;
use crate::model::AgentDefinitionDiagnostic;
use crate::model::AgentDefinitionDiagnosticCode;
use crate::model::AgentDefinitionFields;
use serde::Deserialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

const AGENT_DIRECTORY: &str = ".zeta/agents";
const MAX_ENTRIES: usize = 64;
const MAX_FILE_BYTES: usize = 32 * 1024;
const MAX_DESCRIPTION_BYTES: usize = 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AgentFrontmatter {
    name: String,
    description: String,
    model: Option<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    instructions: Vec<String>,
}

/// Refreshable catalog for one Workspace's native Agent definition directory.
pub struct AgentDefinitionCatalog {
    workspace_root: PathBuf,
    snapshot: Arc<AgentDefinitionCatalogSnapshot>,
}

impl AgentDefinitionCatalog {
    pub fn discover(workspace_root: impl AsRef<Path>) -> Self {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let (entries, diagnostics) = scan(&workspace_root);
        Self {
            workspace_root,
            snapshot: Arc::new(AgentDefinitionCatalogSnapshot::new(1, entries, diagnostics)),
        }
    }

    pub fn snapshot(&self) -> Arc<AgentDefinitionCatalogSnapshot> {
        Arc::clone(&self.snapshot)
    }

    pub fn refresh(&mut self) -> Arc<AgentDefinitionCatalogSnapshot> {
        let (entries, diagnostics) = scan(&self.workspace_root);
        if self.snapshot.entries() == entries && self.snapshot.diagnostics() == diagnostics {
            return Arc::clone(&self.snapshot);
        }
        self.snapshot = Arc::new(AgentDefinitionCatalogSnapshot::new(
            self.snapshot
                .generation()
                .checked_add(1)
                .expect("Agent definition catalog generation overflowed"),
            entries,
            diagnostics,
        ));
        Arc::clone(&self.snapshot)
    }
}

fn scan(workspace_root: &Path) -> (Vec<AgentDefinition>, Vec<AgentDefinitionDiagnostic>) {
    let source_root = workspace_root.join(AGENT_DIRECTORY);
    let metadata = match fs::symlink_metadata(&source_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return (Vec::new(), Vec::new()),
        Err(_) => {
            return (
                Vec::new(),
                vec![diagnostic(
                    None,
                    AgentDefinitionDiagnosticCode::SourceUnavailable,
                    "Workspace Agent definition directory metadata is unavailable",
                )],
            );
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return (
            Vec::new(),
            vec![diagnostic(
                None,
                AgentDefinitionDiagnosticCode::SourceUnavailable,
                "Workspace Agent definition path must be a real directory",
            )],
        );
    }
    let mut paths = match fs::read_dir(&source_root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>(),
        Err(_) => {
            return (
                Vec::new(),
                vec![diagnostic(
                    None,
                    AgentDefinitionDiagnosticCode::SourceUnavailable,
                    "Workspace Agent definition directory cannot be read",
                )],
            );
        }
    };
    paths.sort();
    let mut diagnostics = Vec::new();
    if paths.len() > MAX_ENTRIES {
        diagnostics.push(diagnostic(
            None,
            AgentDefinitionDiagnosticCode::EntryLimitExceeded,
            format!("only the first {MAX_ENTRIES} Agent definitions are inspected"),
        ));
        paths.truncate(MAX_ENTRIES);
    }
    let mut entries = paths
        .into_iter()
        .filter_map(|path| load_entry(&source_root, &path, &mut diagnostics))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name().cmp(right.name()));
    diagnostics.sort_by(|left, right| {
        (left.relative_path(), left.code(), left.message()).cmp(&(
            right.relative_path(),
            right.code(),
            right.message(),
        ))
    });
    (entries, diagnostics)
}

fn load_entry(
    source_root: &Path,
    path: &Path,
    diagnostics: &mut Vec<AgentDefinitionDiagnostic>,
) -> Option<AgentDefinition> {
    let relative_path = path.strip_prefix(source_root).ok()?.to_path_buf();
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::SourceUnavailable,
                "Agent definition metadata is unavailable",
            ));
            return None;
        }
    };
    if metadata.file_type().is_symlink() {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::SymlinkNotAllowed,
            "Agent definitions cannot be symbolic links",
        ));
        return None;
    }
    if !metadata.is_file() || path.extension().and_then(|value| value.to_str()) != Some("md") {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::UnsupportedFileType,
            "Agent definitions must be direct .md files",
        ));
        return None;
    }
    if metadata.len() > MAX_FILE_BYTES as u64 {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::ContentTooLarge,
            format!("Agent definition exceeds {MAX_FILE_BYTES} bytes"),
        ));
        return None;
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::SourceUnavailable,
                "Agent definition content cannot be read",
            ));
            return None;
        }
    };
    let content_digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::ContentInvalidUtf8,
                "Agent definition content must be UTF-8",
            ));
            return None;
        }
    };
    let (frontmatter, body) = match split_frontmatter(&text) {
        Some(parts) => parts,
        None => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::InvalidFrontmatter,
                "Agent definition must start with YAML frontmatter",
            ));
            return None;
        }
    };
    let frontmatter: AgentFrontmatter = match serde_yaml::from_str(frontmatter) {
        Ok(frontmatter) => frontmatter,
        Err(_) => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::InvalidFrontmatter,
                "Agent definition frontmatter is invalid",
            ));
            return None;
        }
    };
    let file_name = path.file_stem()?.to_str()?;
    if !valid_name(&frontmatter.name) || frontmatter.name != file_name {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::InvalidName,
            "Agent name must match its lowercase filename",
        ));
        return None;
    }
    let description = frontmatter.description.trim().to_owned();
    if description.is_empty() || description.len() > MAX_DESCRIPTION_BYTES {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::DescriptionInvalid,
            "Agent description must contain 1 to 1024 UTF-8 bytes",
        ));
        return None;
    }
    let model = match frontmatter.model {
        Some(model) if valid_reference(&model) => Some(model),
        Some(_) => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::InvalidReference,
                "Agent model reference is invalid",
            ));
            return None;
        }
        None => None,
    };
    let tools = validate_references(frontmatter.tools);
    let skills = validate_references(frontmatter.skills);
    let instructions = validate_references(frontmatter.instructions);
    let (tools, skills, instructions) = match (tools, skills, instructions) {
        (Some(tools), Some(skills), Some(instructions)) => (tools, skills, instructions),
        _ => {
            diagnostics.push(diagnostic(
                Some(relative_path),
                AgentDefinitionDiagnosticCode::InvalidReference,
                "Agent tool, Skill, and Instruction references must be valid and unique",
            ));
            return None;
        }
    };
    let role_instructions = body.trim().to_owned();
    if role_instructions.is_empty() {
        diagnostics.push(diagnostic(
            Some(relative_path),
            AgentDefinitionDiagnosticCode::EmptyBody,
            "Agent role instructions cannot be empty",
        ));
        return None;
    }
    Some(AgentDefinition::new(AgentDefinitionFields {
        name: frontmatter.name,
        description,
        content_digest,
        relative_path,
        model,
        tools,
        skills,
        instructions,
        role_instructions,
    }))
}

fn validate_references(values: Vec<String>) -> Option<Vec<String>> {
    if values.iter().any(|value| !valid_reference(value)) {
        return None;
    }
    let unique = values.iter().collect::<BTreeSet<_>>();
    (unique.len() == values.len()).then_some(values)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("---\n")?;
    let boundary = rest.find("\n---\n")?;
    Some((&rest[..boundary], &rest[boundary + 5..]))
}

fn diagnostic(
    relative_path: Option<PathBuf>,
    code: AgentDefinitionDiagnosticCode,
    message: impl Into<String>,
) -> AgentDefinitionDiagnostic {
    AgentDefinitionDiagnostic::new(relative_path, code, message)
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
