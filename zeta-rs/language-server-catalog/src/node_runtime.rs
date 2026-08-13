use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use zeta_install_context::InstallContext;
use zeta_lsp::LanguageServerCommand;

use crate::LanguageServerProviderError;
use crate::provider::canonical_executable;
use crate::provider::canonical_regular_file;

const SAFE_NODE_ENVIRONMENT: &[&str] = &[
    "HOME",
    "LANG",
    "LC_ALL",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "TMPDIR",
    "TZ",
    "USERPROFILE",
    "WINDIR",
];

/// One frozen Node executable supplied and versioned by the Zeta installation.
///
/// The runtime never resolves `node` from the ambient `PATH`. Packaged hosts use
/// [`Self::from_install_context`]; development hosts and tests may inject an exact executable with
/// [`Self::from_path`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedNodeRuntime {
    executable: PathBuf,
}

impl ManagedNodeRuntime {
    /// Freezes one exact managed Node executable after regular-file and executable validation.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, LanguageServerProviderError> {
        Ok(Self {
            executable: canonical_executable(path.as_ref(), "managed Node runtime")?,
        })
    }

    /// Resolves Node only from the signed Zeta package resource layout.
    pub fn from_install_context(
        context: &InstallContext,
    ) -> Result<Self, LanguageServerProviderError> {
        let resource = PathBuf::from("node")
            .join("bin")
            .join(node_executable_name());
        let path = context
            .bundled_resource(&resource)
            .ok_or(LanguageServerProviderError::ManagedNodeUnavailable)?;
        Self::from_path(path)
    }

    /// Returns the frozen canonical Node executable.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub(crate) fn command_for_script(
        &self,
        script: &Path,
        workspace_root: &Path,
    ) -> Result<LanguageServerCommand, LanguageServerProviderError> {
        let script = canonical_regular_file(script, "language-server Node entrypoint")?;
        let mut command = LanguageServerCommand::new(self.executable.as_os_str())
            .with_clean_environment()
            .with_argument(script.into_os_string())
            .with_argument("--stdio")
            .with_current_dir(workspace_root);
        for name in SAFE_NODE_ENVIRONMENT {
            if let Some(value) = env::var_os(name) {
                command = command.with_environment(name, value);
            }
        }
        Ok(command)
    }
}

#[cfg(windows)]
fn node_executable_name() -> &'static OsStr {
    OsStr::new("node.exe")
}

#[cfg(not(windows))]
fn node_executable_name() -> &'static OsStr {
    OsStr::new("node")
}
