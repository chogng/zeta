/// The immutable prompt additions captured for one Workspace runtime.
///
/// Hosts construct this value at a Turn boundary and reuse it for every model invocation in the
/// runtime. The environment and workspace instructions therefore remain stable until the host
/// explicitly creates a new runtime or a future compaction boundary refreshes them.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessInstructions {
    system_body: String,
    environment: String,
    workspace_instructions: Option<String>,
}

impl HarnessInstructions {
    /// Creates the prompt additions from a system body, rendered environment block, and optional
    /// workspace instructions. The caller owns discovery and truncation of AGENTS.md.
    pub fn new(
        system_body: impl Into<String>,
        environment: impl Into<String>,
        workspace_instructions: Option<String>,
    ) -> Self {
        Self {
            system_body: system_body.into(),
            environment: environment.into(),
            workspace_instructions,
        }
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

    pub(crate) fn model_instructions(&self) -> Option<String> {
        match (
            self.system_body.trim().is_empty(),
            self.environment.trim().is_empty(),
        ) {
            (true, true) => None,
            (true, false) => Some(self.environment.clone()),
            (false, true) => Some(self.system_body.clone()),
            (false, false) => Some(format!("{}\n\n{}", self.system_body, self.environment)),
        }
    }

    pub(crate) fn workspace_message(&self) -> Option<String> {
        self.workspace_instructions.as_ref().map(|instructions| {
            format!(
                "<workspace-instructions>\nWorkspace instructions from AGENTS.md. They rank below system and safety policy.\n{}\n</workspace-instructions>",
                instructions
            )
        })
    }
}
