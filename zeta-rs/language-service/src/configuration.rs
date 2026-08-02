use std::path::{Path, PathBuf};

use crate::{LanguageServerDefinition, LanguageServerRestartPolicy};

/// Product policy controlling whether configured language servers may run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LanguageServiceEnablement {
    /// Retain document snapshots without starting or contacting language servers.
    #[default]
    Disabled,
    /// Start the caller-resolved server catalog and synchronize matching documents.
    Enabled,
}

/// Immutable product configuration consumed by one language-service supervisor.
#[derive(Clone, Debug)]
pub struct LanguageServiceConfiguration {
    pub(crate) workspace_root: PathBuf,
    pub(crate) enablement: LanguageServiceEnablement,
    pub(crate) servers: Vec<LanguageServerDefinition>,
    pub(crate) restart_policy: LanguageServerRestartPolicy,
}

impl LanguageServiceConfiguration {
    pub fn disabled(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            enablement: LanguageServiceEnablement::Disabled,
            servers: Vec::new(),
            restart_policy: LanguageServerRestartPolicy::standard(),
        }
    }

    pub fn enabled(
        workspace_root: impl Into<PathBuf>,
        servers: Vec<LanguageServerDefinition>,
    ) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            enablement: LanguageServiceEnablement::Enabled,
            servers,
            restart_policy: LanguageServerRestartPolicy::standard(),
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub const fn enablement(&self) -> LanguageServiceEnablement {
        self.enablement
    }

    pub fn servers(&self) -> &[LanguageServerDefinition] {
        &self.servers
    }

    pub const fn restart_policy(&self) -> LanguageServerRestartPolicy {
        self.restart_policy
    }

    pub const fn with_restart_policy(mut self, policy: LanguageServerRestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }
}
