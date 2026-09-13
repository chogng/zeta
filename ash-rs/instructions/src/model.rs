use std::path::PathBuf;
use std::sync::Arc;

/// Canonical policy controlling when one Instruction contributes model-facing content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstructionLoadPolicy {
    Global,
    Contextual { patterns: Vec<String> },
    OnDemand,
}

/// One validated native Directory Instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionArtifact {
    name: String,
    relative_path: PathBuf,
    load_policy: InstructionLoadPolicy,
    body: String,
}

impl InstructionArtifact {
    pub(crate) fn new(
        name: String,
        relative_path: PathBuf,
        load_policy: InstructionLoadPolicy,
        body: String,
    ) -> Self {
        Self {
            name,
            relative_path,
            load_policy,
            body,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn relative_path(&self) -> &std::path::Path {
        &self.relative_path
    }

    pub fn load_policy(&self) -> &InstructionLoadPolicy {
        &self.load_policy
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum InstructionDiagnosticCode {
    SourceUnavailable,
    EntryLimitExceeded,
    UnsupportedFileType,
    SymlinkNotAllowed,
    InvalidName,
    InvalidFrontmatter,
    InvalidLoadPolicy,
    ContentTooLarge,
    ContentInvalidUtf8,
    EmptyBody,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionDiagnostic {
    relative_path: Option<PathBuf>,
    code: InstructionDiagnosticCode,
    message: String,
}

impl InstructionDiagnostic {
    pub(crate) fn new(
        relative_path: Option<PathBuf>,
        code: InstructionDiagnosticCode,
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

    pub fn code(&self) -> InstructionDiagnosticCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InstructionCatalogSnapshot {
    generation: u64,
    entries: Arc<[InstructionArtifact]>,
    diagnostics: Arc<[InstructionDiagnostic]>,
}

impl InstructionCatalogSnapshot {
    pub(crate) fn new(
        generation: u64,
        entries: Vec<InstructionArtifact>,
        diagnostics: Vec<InstructionDiagnostic>,
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

    pub fn entries(&self) -> &[InstructionArtifact] {
        &self.entries
    }

    pub fn diagnostics(&self) -> &[InstructionDiagnostic] {
        &self.diagnostics
    }

    /// Renders all Global Instructions in deterministic catalog order.
    pub fn global_content(&self) -> Option<String> {
        let content = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.load_policy(), InstructionLoadPolicy::Global))
            .map(|entry| {
                format!(
                    "<instruction name=\"{}\" source=\"{}\">\n{}\n</instruction>",
                    entry.name(),
                    entry.relative_path().display(),
                    entry.body()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        (!content.is_empty()).then_some(content)
    }
}
