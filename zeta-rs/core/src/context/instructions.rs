use super::InstructionFragment;
use super::InstructionLayer;
use super::InstructionRetention;
use super::InstructionSource;

/// Immutable system and directory instructions supplied by the host.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessInstructions {
    system_body: String,
    system_revision: String,
    directory_instructions: Option<String>,
    directory_revision: String,
}

impl HarnessInstructions {
    /// Creates prompt additions from a system body and optional directory instructions.
    pub fn new(system_body: impl Into<String>, directory_instructions: Option<String>) -> Self {
        Self {
            system_body: system_body.into(),
            system_revision: "unversioned-system".into(),
            directory_instructions,
            directory_revision: "unversioned-directory".into(),
        }
    }

    /// Creates host additions containing only optional directory instructions.
    pub fn directory(directory_instructions: Option<String>) -> Self {
        Self::new(String::new(), directory_instructions)
    }

    pub fn with_system_revision(mut self, revision: impl Into<String>) -> Self {
        self.system_revision = revision.into();
        self
    }

    pub fn with_directory_revision(mut self, revision: impl Into<String>) -> Self {
        self.directory_revision = revision.into();
        self
    }

    pub fn system_body(&self) -> &str {
        &self.system_body
    }

    pub fn directory_instructions(&self) -> Option<&str> {
        self.directory_instructions.as_deref()
    }

    pub(crate) fn context_fragments(&self) -> Vec<InstructionFragment> {
        let mut fragments = Vec::new();
        if !self.system_body.trim().is_empty() {
            fragments.push(InstructionFragment::new(
                InstructionSource::new(
                    "system",
                    "zeta-system-prompt",
                    self.system_revision.clone(),
                ),
                InstructionLayer::System,
                InstructionRetention::Required,
                self.system_body.clone(),
            ));
        }
        if let Some(instructions) = self
            .directory_instructions
            .as_ref()
            .filter(|instructions| !instructions.trim().is_empty())
        {
            fragments.push(InstructionFragment::new(
                InstructionSource::new(
                    "directory",
                    "directory-instructions",
                    self.directory_revision.clone(),
                ),
                InstructionLayer::Directory,
                InstructionRetention::Required,
                format!(
                    "<directory-instructions>\nDirectory Instructions from .zeta/instructions. They rank below system and safety policy.\n{}\n</directory-instructions>",
                    instructions
                ),
            ));
        }
        fragments
    }
}
