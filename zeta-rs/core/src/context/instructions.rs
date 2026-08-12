use super::InstructionFragment;
use super::InstructionLayer;
use super::InstructionRetention;
use super::InstructionSource;
use std::sync::Arc;

/// The immutable prompt additions captured for one Workspace runtime.
///
/// Hosts construct or refresh this value only at a model-invocation boundary. One value remains
/// immutable while its request is planned and assembled; a provider may return a newer value at
/// the next safe point without changing an in-flight invocation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessInstructions {
    system_body: String,
    system_revision: String,
    environment: String,
    environment_revision: String,
    workspace_instructions: Option<String>,
    workspace_revision: String,
}

impl HarnessInstructions {
    /// Creates the prompt additions from a system body, rendered environment block, and optional
    /// Workspace instructions. The caller owns discovery, validation, and content bounds.
    pub fn new(
        system_body: impl Into<String>,
        environment: impl Into<String>,
        workspace_instructions: Option<String>,
    ) -> Self {
        Self {
            system_body: system_body.into(),
            system_revision: "unversioned-system".into(),
            environment: environment.into(),
            environment_revision: "runtime-snapshot".into(),
            workspace_instructions,
            workspace_revision: "unversioned-workspace".into(),
        }
    }

    pub fn with_system_revision(mut self, revision: impl Into<String>) -> Self {
        self.system_revision = revision.into();
        self
    }

    pub fn with_environment_revision(mut self, revision: impl Into<String>) -> Self {
        self.environment_revision = revision.into();
        self
    }

    pub fn with_workspace_revision(mut self, revision: impl Into<String>) -> Self {
        self.workspace_revision = revision.into();
        self
    }

    pub fn system_body(&self) -> &str {
        &self.system_body
    }

    pub fn environment(&self) -> &str {
        &self.environment
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
        if !self.environment.trim().is_empty() {
            fragments.push(InstructionFragment::new(
                InstructionSource::new(
                    "environment",
                    "workspace-environment",
                    self.environment_revision.clone(),
                ),
                InstructionLayer::Product,
                InstructionRetention::Required,
                self.environment.clone(),
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

/// Supplies an immutable Instruction snapshot at each model-invocation boundary.
///
/// Implementations may refresh between invocations, but one returned snapshot must remain stable
/// for the complete request assembled from it.
pub trait HarnessInstructionsProvider: Send + Sync {
    fn snapshot(&self) -> Arc<HarnessInstructions>;
}
