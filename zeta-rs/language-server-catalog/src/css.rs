use std::path::Path;

use zeta_language_server_distribution::InstalledLanguageServer;
use zeta_lsp::LanguageServerCommand;

use crate::CSS_LANGUAGE_SERVER_ID;
use crate::LanguageServerDefinition;
use crate::LanguageServerProvider;
use crate::LanguageServerProviderError;
use crate::LanguageServerProviderLaunch;
use crate::ManagedNodeRuntime;
use crate::provider::canonical_executable;
use crate::provider::canonical_regular_file;

const CSS_LANGUAGE_IDS: &[&str] = &["css", "less", "scss"];

/// Provider for the verified Marketplace CSS package executed by Zeta's managed Node runtime.
pub struct CssLanguageServerProvider {
    entrypoint: std::path::PathBuf,
    node: ManagedNodeRuntime,
}

impl CssLanguageServerProvider {
    /// Binds one verified CSS installation receipt to the shared managed Node runtime.
    pub fn new(
        installed: InstalledLanguageServer,
        node: ManagedNodeRuntime,
    ) -> Result<Self, LanguageServerProviderError> {
        if installed.server_id() != CSS_LANGUAGE_SERVER_ID {
            return Err(LanguageServerProviderError::InstalledServerIdentity {
                expected: CSS_LANGUAGE_SERVER_ID,
                actual: installed.server_id().to_owned(),
            });
        }
        Ok(Self {
            entrypoint: canonical_regular_file(
                installed.executable(),
                "installed CSS language-server entrypoint",
            )?,
            node,
        })
    }

    /// Returns the canonical installed JavaScript entrypoint.
    pub fn entrypoint(&self) -> &Path {
        &self.entrypoint
    }

    /// Returns the exact Node runtime used for packaged launches.
    pub fn node_runtime(&self) -> &ManagedNodeRuntime {
        &self.node
    }
}

impl LanguageServerProvider for CssLanguageServerProvider {
    fn id(&self) -> &'static str {
        CSS_LANGUAGE_SERVER_ID
    }

    fn languages(&self) -> &'static [&'static str] {
        CSS_LANGUAGE_IDS
    }

    fn definition(
        &self,
        workspace_root: &Path,
        launch: LanguageServerProviderLaunch<'_>,
    ) -> Result<LanguageServerDefinition, LanguageServerProviderError> {
        let command = match launch {
            LanguageServerProviderLaunch::Packaged => self
                .node
                .command_for_script(&self.entrypoint, workspace_root)?,
            LanguageServerProviderLaunch::ExplicitExecutable(executable) => {
                LanguageServerCommand::new(canonical_executable(
                    executable,
                    "explicit CSS language-server executable",
                )?)
                .with_current_dir(workspace_root)
            }
        };
        LanguageServerDefinition::new(
            CSS_LANGUAGE_SERVER_ID,
            CSS_LANGUAGE_IDS.iter().copied(),
            command,
        )
        .map_err(Into::into)
    }
}
