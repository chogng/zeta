use std::fs;
use std::path::{Path, PathBuf};

use zeta_install_context::{HostExecutableName, InstallContext};
use zeta_lsp::LanguageServerCommand;

use crate::{
    BASH_LANGUAGE_SERVER_ID, JSON_LANGUAGE_SERVER_ID, LanguageServerCatalogError,
    LanguageServerDefinition, RUST_ANALYZER_SERVER_ID,
};

/// Supplies frozen executable candidates without granting authority to start them.
///
/// Implementations should preserve source precedence and must not perform process execution.
/// The catalog validates and canonicalizes every returned candidate before producing a launch
/// definition.
pub trait LanguageServerExecutableCandidates {
    fn candidates(&self, executable_name: &str)
    -> Result<Vec<PathBuf>, LanguageServerCatalogError>;
}

impl LanguageServerExecutableCandidates for InstallContext {
    fn candidates(
        &self,
        executable_name: &str,
    ) -> Result<Vec<PathBuf>, LanguageServerCatalogError> {
        let name = HostExecutableName::new(executable_name)?;
        Ok(self.host_path_candidates(&name))
    }
}

/// User intent for one built-in language server.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LanguageServerMode {
    Disabled,
    #[default]
    Automatic,
    Enabled,
}

/// Product policy governing whether catalog resolution may produce executable definitions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LanguageServerExecutionPolicy {
    #[default]
    Disallowed,
    Allowed,
}

/// Desired mode and executable source for one language server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerPreference {
    mode: LanguageServerMode,
    explicit_executable: Option<PathBuf>,
}

impl LanguageServerPreference {
    pub fn disabled() -> Self {
        Self {
            mode: LanguageServerMode::Disabled,
            explicit_executable: None,
        }
    }

    pub fn automatic() -> Self {
        Self {
            mode: LanguageServerMode::Automatic,
            explicit_executable: None,
        }
    }

    pub fn enabled() -> Self {
        Self {
            mode: LanguageServerMode::Enabled,
            explicit_executable: None,
        }
    }

    pub fn with_explicit_executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_executable = Some(path.into());
        self
    }

    pub const fn mode(&self) -> LanguageServerMode {
        self.mode
    }
}

impl Default for LanguageServerPreference {
    fn default() -> Self {
        Self::automatic()
    }
}

/// Resolution state of one built-in server for the current workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageServerCatalogState {
    Disabled,
    ExecutionDisallowed,
    ExecutableUnavailable,
    Resolved { executable: PathBuf },
}

/// Product-visible catalog entry without any live runtime state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerCatalogEntry {
    name: &'static str,
    mode: LanguageServerMode,
    state: LanguageServerCatalogState,
}

impl LanguageServerCatalogEntry {
    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn mode(&self) -> LanguageServerMode {
        self.mode
    }

    pub fn state(&self) -> &LanguageServerCatalogState {
        &self.state
    }
}

/// Frozen resolved definitions and availability facts for one workspace.
#[derive(Clone, Debug)]
pub struct LanguageServerCatalogResolution {
    definitions: Vec<LanguageServerDefinition>,
    entries: Vec<LanguageServerCatalogEntry>,
}

impl LanguageServerCatalogResolution {
    pub fn definitions(&self) -> &[LanguageServerDefinition] {
        &self.definitions
    }

    pub fn entries(&self) -> &[LanguageServerCatalogEntry] {
        &self.entries
    }

    pub fn into_definitions(self) -> Vec<LanguageServerDefinition> {
        self.definitions
    }
}

/// Built-in language-server catalog and its user-selected preferences.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LanguageServerCatalog {
    rust_analyzer: LanguageServerPreference,
    json_language_server: LanguageServerPreference,
    bash_language_server: LanguageServerPreference,
}

impl LanguageServerCatalog {
    pub fn new(rust_analyzer: LanguageServerPreference) -> Self {
        Self {
            rust_analyzer,
            ..Self::default()
        }
    }

    pub fn with_json_language_server(mut self, preference: LanguageServerPreference) -> Self {
        self.json_language_server = preference;
        self
    }

    pub fn with_bash_language_server(mut self, preference: LanguageServerPreference) -> Self {
        self.bash_language_server = preference;
        self
    }

    pub fn resolve(
        &self,
        executable_candidates: &dyn LanguageServerExecutableCandidates,
        execution_policy: LanguageServerExecutionPolicy,
        workspace_root: &Path,
    ) -> Result<LanguageServerCatalogResolution, LanguageServerCatalogError> {
        let mut definitions = Vec::new();
        let rust_state = self.resolve_builtin(
            BuiltinServer::rust_analyzer(),
            &self.rust_analyzer,
            executable_candidates,
            execution_policy,
            workspace_root,
            &mut definitions,
        )?;
        let json_state = self.resolve_builtin(
            BuiltinServer::json(),
            &self.json_language_server,
            executable_candidates,
            execution_policy,
            workspace_root,
            &mut definitions,
        )?;
        let bash_state = self.resolve_builtin(
            BuiltinServer::bash(),
            &self.bash_language_server,
            executable_candidates,
            execution_policy,
            workspace_root,
            &mut definitions,
        )?;
        Ok(LanguageServerCatalogResolution {
            definitions,
            entries: vec![
                LanguageServerCatalogEntry {
                    name: RUST_ANALYZER_SERVER_ID,
                    mode: self.rust_analyzer.mode,
                    state: rust_state,
                },
                LanguageServerCatalogEntry {
                    name: JSON_LANGUAGE_SERVER_ID,
                    mode: self.json_language_server.mode,
                    state: json_state,
                },
                LanguageServerCatalogEntry {
                    name: BASH_LANGUAGE_SERVER_ID,
                    mode: self.bash_language_server.mode,
                    state: bash_state,
                },
            ],
        })
    }

    fn resolve_builtin(
        &self,
        builtin: BuiltinServer,
        preference: &LanguageServerPreference,
        executable_candidates: &dyn LanguageServerExecutableCandidates,
        execution_policy: LanguageServerExecutionPolicy,
        workspace_root: &Path,
        definitions: &mut Vec<LanguageServerDefinition>,
    ) -> Result<LanguageServerCatalogState, LanguageServerCatalogError> {
        if preference.mode == LanguageServerMode::Disabled {
            return Ok(LanguageServerCatalogState::Disabled);
        }
        if execution_policy == LanguageServerExecutionPolicy::Disallowed {
            return Ok(LanguageServerCatalogState::ExecutionDisallowed);
        }
        let executable = if let Some(explicit) = &preference.explicit_executable {
            valid_executable(explicit)
        } else {
            executable_candidates
                .candidates(builtin.executable)?
                .into_iter()
                .find_map(|candidate| valid_executable(&candidate))
        };
        let Some(executable) = executable else {
            return Ok(LanguageServerCatalogState::ExecutableUnavailable);
        };
        let command = LanguageServerCommand::new(executable.clone())
            .with_arguments(builtin.arguments)
            .with_current_dir(workspace_root.to_path_buf());
        definitions.push(LanguageServerDefinition::new(
            builtin.identity,
            builtin.language_ids.iter().copied(),
            command,
        )?);
        Ok(LanguageServerCatalogState::Resolved { executable })
    }
}

#[derive(Clone, Copy)]
struct BuiltinServer {
    identity: &'static str,
    executable: &'static str,
    language_ids: &'static [&'static str],
    arguments: &'static [&'static str],
}

impl BuiltinServer {
    const fn rust_analyzer() -> Self {
        Self {
            identity: RUST_ANALYZER_SERVER_ID,
            executable: RUST_ANALYZER_SERVER_ID,
            language_ids: &["rust"],
            arguments: &[],
        }
    }

    const fn json() -> Self {
        Self {
            identity: JSON_LANGUAGE_SERVER_ID,
            executable: JSON_LANGUAGE_SERVER_ID,
            language_ids: &["json", "jsonc"],
            arguments: &["--stdio"],
        }
    }

    const fn bash() -> Self {
        Self {
            identity: BASH_LANGUAGE_SERVER_ID,
            executable: BASH_LANGUAGE_SERVER_ID,
            language_ids: &["shellscript"],
            arguments: &["start"],
        }
    }
}

fn valid_executable(path: &Path) -> Option<PathBuf> {
    let canonical = fs::canonicalize(path).ok()?;
    let metadata = fs::metadata(&canonical).ok()?;
    if !metadata.is_file() || !has_executable_permission(&metadata) {
        return None;
    }
    Some(canonical)
}

#[cfg(unix)]
fn has_executable_permission(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_executable_permission(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(test)]
#[path = "resolver_tests.rs"]
mod tests;
