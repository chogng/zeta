use std::path::PathBuf;
use std::sync::Arc;

/// One validated Agent execution configuration declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDefinition {
    name: String,
    description: String,
    content_digest: String,
    relative_path: PathBuf,
    model: Option<String>,
    tools: Vec<String>,
    skills: Vec<String>,
    instructions: Vec<String>,
    role_instructions: String,
}

pub(crate) struct AgentDefinitionFields {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) content_digest: String,
    pub(crate) relative_path: PathBuf,
    pub(crate) model: Option<String>,
    pub(crate) tools: Vec<String>,
    pub(crate) skills: Vec<String>,
    pub(crate) instructions: Vec<String>,
    pub(crate) role_instructions: String,
}

impl AgentDefinition {
    pub(crate) fn new(fields: AgentDefinitionFields) -> Self {
        Self {
            name: fields.name,
            description: fields.description,
            content_digest: fields.content_digest,
            relative_path: fields.relative_path,
            model: fields.model,
            tools: fields.tools,
            skills: fields.skills,
            instructions: fields.instructions,
            role_instructions: fields.role_instructions,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn relative_path(&self) -> &std::path::Path {
        &self.relative_path
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn tools(&self) -> &[String] {
        &self.tools
    }

    pub fn skills(&self) -> &[String] {
        &self.skills
    }

    pub fn instructions(&self) -> &[String] {
        &self.instructions
    }

    pub fn role_instructions(&self) -> &str {
        &self.role_instructions
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AgentDefinitionDiagnosticCode {
    SourceUnavailable,
    EntryLimitExceeded,
    UnsupportedFileType,
    SymlinkNotAllowed,
    InvalidName,
    InvalidFrontmatter,
    DescriptionInvalid,
    InvalidReference,
    ContentTooLarge,
    ContentInvalidUtf8,
    EmptyBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDefinitionDiagnostic {
    relative_path: Option<PathBuf>,
    code: AgentDefinitionDiagnosticCode,
    message: String,
}

impl AgentDefinitionDiagnostic {
    pub(crate) fn new(
        relative_path: Option<PathBuf>,
        code: AgentDefinitionDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            relative_path,
            code,
            message: message.into(),
        }
    }

    pub fn relative_path(&self) -> Option<&std::path::Path> {
        self.relative_path.as_deref()
    }

    pub fn code(&self) -> AgentDefinitionDiagnosticCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentDefinitionCatalogSnapshot {
    generation: u64,
    entries: Arc<[AgentDefinition]>,
    diagnostics: Arc<[AgentDefinitionDiagnostic]>,
}

impl AgentDefinitionCatalogSnapshot {
    pub(crate) fn new(
        generation: u64,
        entries: Vec<AgentDefinition>,
        diagnostics: Vec<AgentDefinitionDiagnostic>,
    ) -> Self {
        Self {
            generation,
            entries: entries.into(),
            diagnostics: diagnostics.into(),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn entries(&self) -> &[AgentDefinition] {
        &self.entries
    }

    pub fn diagnostics(&self) -> &[AgentDefinitionDiagnostic] {
        &self.diagnostics
    }
}
