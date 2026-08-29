use super::InstructionFragment;
use super::InstructionLayer;
use super::InstructionRetention;
use super::InstructionSource;

/// Immutable system and Workspace instructions supplied by the host.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessInstructions {
    system_body: String,
    system_revision: String,
    workspace_instructions: Option<String>,
    workspace_revision: String,
}

impl HarnessInstructions {
    /// Creates prompt additions from a system body and optional Workspace instructions.
    pub fn new(system_body: impl Into<String>, workspace_instructions: Option<String>) -> Self {
        Self {
            system_body: system_body.into(),
            system_revision: "unversioned-system".into(),
            workspace_instructions,
            workspace_revision: "unversioned-workspace".into(),
        }
    }

    /// Creates host additions containing only optional Workspace instructions.
    pub fn workspace(workspace_instructions: Option<String>) -> Self {
        Self::new(String::new(), workspace_instructions)
    }

    pub fn with_system_revision(mut self, revision: impl Into<String>) -> Self {
        self.system_revision = revision.into();
        self
    }

    pub fn with_workspace_revision(mut self, revision: impl Into<String>) -> Self {
        self.workspace_revision = revision.into();
        self
    }

    pub fn system_body(&self) -> &str {
        &self.system_body
    }

    pub fn workspace_instructions(&self) -> Option<&str> {
        self.workspace_instructions.as_deref()
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
            .workspace_instructions
            .as_ref()
            .filter(|instructions| !instructions.trim().is_empty())
        {
            fragments.push(InstructionFragment::new(
                InstructionSource::new(
                    "workspace",
                    "global-workspace-instructions",
                    self.workspace_revision.clone(),
                ),
                InstructionLayer::Workspace,
                InstructionRetention::Required,
                format!(
                    "<workspace-instructions>\nGlobal Workspace Instructions from .zeta/instructions. They rank below system and safety policy.\n{}\n</workspace-instructions>",
                    instructions
                ),
            ));
        }
        fragments
    }
}
