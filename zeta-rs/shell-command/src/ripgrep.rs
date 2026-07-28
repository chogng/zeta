use crate::ShellCommandRequest;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

const RIPGREP_OVERRIDE: &str = "ZETA_RG_PATH";

/// Frozen absolute identity of the ripgrep executable selected at host startup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RipgrepExecutable {
    path: PathBuf,
}

impl RipgrepExecutable {
    /// Resolves ripgrep from `ZETA_RG_PATH`, beside the current executable, or from `PATH`.
    ///
    /// The selected path is canonicalized once so later Tool Calls cannot change executable
    /// identity by mutating `PATH`.
    pub fn discover() -> Result<Self, RipgrepDiscoveryError> {
        if let Some(path) = env::var_os(RIPGREP_OVERRIDE) {
            return Self::from_path(path).map_err(|error| RipgrepDiscoveryError::InvalidOverride {
                variable: RIPGREP_OVERRIDE,
                reason: error.to_string(),
            });
        }

        if let Ok(current_executable) = env::current_exe()
            && let Some(directory) = current_executable.parent()
            && let Some(executable) = discover_in_directories([directory])
        {
            return Ok(executable);
        }
        let Some(path) = env::var_os("PATH") else {
            return Err(RipgrepDiscoveryError::NotFound);
        };
        discover_in_directories(env::split_paths(&path)).ok_or(RipgrepDiscoveryError::NotFound)
    }

    /// Freezes one explicit ripgrep executable path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, RipgrepDiscoveryError> {
        let path = path.as_ref();
        let metadata = path
            .metadata()
            .map_err(|error| RipgrepDiscoveryError::InvalidExecutable(error.to_string()))?;
        if !metadata.is_file() {
            return Err(RipgrepDiscoveryError::InvalidExecutable(
                "ripgrep candidate is not a regular file".into(),
            ));
        }
        #[cfg(unix)]
        if std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o111 == 0 {
            return Err(RipgrepDiscoveryError::InvalidExecutable(
                "ripgrep candidate is not executable".into(),
            ));
        }
        let path = path
            .canonicalize()
            .map_err(|error| RipgrepDiscoveryError::InvalidExecutable(error.to_string()))?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Replaces the model-visible `rg` alias with the frozen executable and disables config files.
    pub fn materialize(
        &self,
        request: ShellCommandRequest,
    ) -> Result<ShellCommandRequest, RipgrepRequestError> {
        if request.program() != "rg" {
            return Err(RipgrepRequestError::UnsupportedProgram(
                request.program().to_owned(),
            ));
        }
        for argument in request.arguments() {
            if is_forbidden_argument(argument) {
                return Err(RipgrepRequestError::UnsafeArgument(argument.clone()));
            }
        }
        let mut arguments = Vec::with_capacity(request.arguments().len() + 1);
        arguments.push("--no-config".to_owned());
        arguments.extend(request.arguments().iter().cloned());
        Ok(request
            .replace_program_and_arguments(self.path.to_string_lossy().into_owned(), arguments))
    }
}

fn discover_in_directories(
    directories: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Option<RipgrepExecutable> {
    for directory in directories {
        for name in executable_names() {
            let candidate = directory.as_ref().join(name);
            if let Ok(executable) = RipgrepExecutable::from_path(candidate) {
                return Some(executable);
            }
        }
    }
    None
}

fn executable_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["rg.exe", "rg"]
    } else {
        &["rg"]
    }
}

fn is_forbidden_argument(argument: &str) -> bool {
    argument == "--pre"
        || argument.starts_with("--pre=")
        || argument == "--pre-glob"
        || argument.starts_with("--pre-glob=")
        || argument == "--file"
        || argument.starts_with("--file=")
        || argument == "-f"
        || argument == "--ignore-file"
        || argument.starts_with("--ignore-file=")
        || argument == "--search-zip"
        || argument == "-z"
        || argument == "--follow"
        || argument == "-L"
}

/// Failure to locate and freeze the product ripgrep executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RipgrepDiscoveryError {
    NotFound,
    InvalidExecutable(String),
    InvalidOverride {
        variable: &'static str,
        reason: String,
    },
}

impl fmt::Display for RipgrepDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str(
                "ripgrep was not found beside the Zeta executable or on PATH; set ZETA_RG_PATH",
            ),
            Self::InvalidExecutable(reason) => {
                write!(formatter, "invalid ripgrep executable: {reason}")
            }
            Self::InvalidOverride { variable, reason } => {
                write!(formatter, "{variable} does not identify ripgrep: {reason}")
            }
        }
    }
}

impl std::error::Error for RipgrepDiscoveryError {}

/// Rejection of a command that is not within Zeta's read-only ripgrep profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RipgrepRequestError {
    UnsupportedProgram(String),
    UnsafeArgument(String),
}

impl fmt::Display for RipgrepRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProgram(program) => {
                write!(
                    formatter,
                    "only the frozen `rg` program is available, got `{program}`"
                )
            }
            Self::UnsafeArgument(argument) => {
                write!(formatter, "ripgrep argument is not allowed: {argument}")
            }
        }
    }
}

impl std::error::Error for RipgrepRequestError {}

#[cfg(test)]
#[path = "ripgrep_tests.rs"]
mod tests;
