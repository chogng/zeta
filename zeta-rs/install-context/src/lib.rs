//! Runtime description of the Zeta installation and its managed resource locations.
//!
//! This crate identifies package layout and produces ordered executable candidates. Consumers
//! remain responsible for validating, canonicalizing, probing, and executing those resources.

use std::env;
use std::ffi::OsString;
use std::path::Component;
use std::path::{Path, PathBuf};

const PACKAGE_BIN_DIRECTORY: &str = "bin";
const PACKAGE_PATH_DIRECTORY: &str = "zeta-path";
const PACKAGE_RESOURCES_DIRECTORY: &str = "zeta-resources";
const PACKAGE_METADATA_FILE: &str = "zeta-package.json";
const RIPGREP_OVERRIDE: &str = "ZETA_RG_PATH";
const BUBBLEWRAP_OVERRIDE: &str = "ZETA_BWRAP_PATH";
const WINDOWS_COMMAND_RUNNER_OVERRIDE: &str = "ZETA_WINDOWS_COMMAND_RUNNER_PATH";
const WINDOWS_SANDBOX_SETUP_OVERRIDE: &str = "ZETA_WINDOWS_SANDBOX_SETUP_PATH";

/// Installation shape detected for the running Zeta executable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallMethod {
    /// A package root containing `bin/` and at least one managed resource directory.
    Package,
    /// A development build, custom launcher, or otherwise unrecognized layout.
    Other,
}

/// Directories owned by one packaged Zeta distribution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageLayout {
    package_directory: PathBuf,
    metadata_file: PathBuf,
    binary_directory: PathBuf,
    path_directory: PathBuf,
    resources_directory: PathBuf,
}

impl PackageLayout {
    pub fn package_directory(&self) -> &Path {
        &self.package_directory
    }

    pub fn metadata_file(&self) -> &Path {
        &self.metadata_file
    }

    pub fn binary_directory(&self) -> &Path {
        &self.binary_directory
    }

    pub fn path_directory(&self) -> &Path {
        &self.path_directory
    }

    pub fn resources_directory(&self) -> &Path {
        &self.resources_directory
    }
}

/// Managed executable identities whose installation candidates Zeta can locate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedExecutable {
    Ripgrep,
    Bubblewrap,
    WindowsCommandRunner,
    WindowsSandboxSetup,
}

/// One explicit environment override that must not silently fall back when invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableOverride {
    variable: &'static str,
    path: PathBuf,
}

impl ExecutableOverride {
    pub fn variable(&self) -> &'static str {
        self.variable
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Mutually exclusive resolution paths for one managed executable.
///
/// An explicit override is authoritative and therefore never exposes fallback paths. Without an
/// override, consumers may try [`Self::SearchPaths`] in order and skip invalid candidates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableCandidates {
    ExplicitOverride(ExecutableOverride),
    SearchPaths(Vec<PathBuf>),
}

/// Immutable snapshot of the running Zeta installation and executable search environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallContext {
    method: InstallMethod,
    package_layout: Option<PackageLayout>,
    executable_directory: Option<PathBuf>,
    ripgrep_override: Option<OsString>,
    bubblewrap_override: Option<OsString>,
    windows_command_runner_override: Option<OsString>,
    windows_sandbox_setup_override: Option<OsString>,
    search_path: Option<OsString>,
}

impl InstallContext {
    /// Detects the current executable, package layout, and relevant environment overrides once.
    pub fn current() -> Self {
        Self::detect(
            env::current_exe().ok().as_deref(),
            env::var_os(RIPGREP_OVERRIDE),
            env::var_os(BUBBLEWRAP_OVERRIDE),
            env::var_os(WINDOWS_COMMAND_RUNNER_OVERRIDE),
            env::var_os(WINDOWS_SANDBOX_SETUP_OVERRIDE),
            env::var_os("PATH"),
        )
    }

    pub fn method(&self) -> InstallMethod {
        self.method
    }

    pub fn package_layout(&self) -> Option<&PackageLayout> {
        self.package_layout.as_ref()
    }

    /// Returns an existing file candidate below `zeta-resources/`, when this is a package install.
    pub fn bundled_resource(&self, name: impl AsRef<Path>) -> Option<PathBuf> {
        let name = name.as_ref();
        if !is_safe_resource_name(name) {
            return None;
        }
        let candidate = self.package_layout.as_ref()?.resources_directory.join(name);
        candidate.is_file().then_some(candidate)
    }

    /// Returns an existing directory candidate below `zeta-resources/`.
    ///
    /// Consumers remain responsible for validating the directory tree and must not treat package
    /// provenance as content trust or execution authority.
    pub fn bundled_resource_directory(&self, name: impl AsRef<Path>) -> Option<PathBuf> {
        let name = name.as_ref();
        if !is_safe_resource_name(name) {
            return None;
        }
        let candidate = self.package_layout.as_ref()?.resources_directory.join(name);
        candidate.is_dir().then_some(candidate)
    }

    /// Produces a frozen, precedence-ordered snapshot of installation candidates.
    pub fn executable_candidates(&self, executable: ManagedExecutable) -> ExecutableCandidates {
        let (override_variable, explicit_override) = match executable {
            ManagedExecutable::Ripgrep => (RIPGREP_OVERRIDE, self.ripgrep_override.as_ref()),
            ManagedExecutable::Bubblewrap => {
                (BUBBLEWRAP_OVERRIDE, self.bubblewrap_override.as_ref())
            }
            ManagedExecutable::WindowsCommandRunner => (
                WINDOWS_COMMAND_RUNNER_OVERRIDE,
                self.windows_command_runner_override.as_ref(),
            ),
            ManagedExecutable::WindowsSandboxSetup => (
                WINDOWS_SANDBOX_SETUP_OVERRIDE,
                self.windows_sandbox_setup_override.as_ref(),
            ),
        };
        if let Some(path) = explicit_override {
            return ExecutableCandidates::ExplicitOverride(ExecutableOverride {
                variable: override_variable,
                path: PathBuf::from(path),
            });
        }
        let mut paths = Vec::new();
        if let Some(layout) = &self.package_layout {
            let directory = match executable {
                ManagedExecutable::Ripgrep => &layout.path_directory,
                ManagedExecutable::Bubblewrap
                | ManagedExecutable::WindowsCommandRunner
                | ManagedExecutable::WindowsSandboxSetup => &layout.resources_directory,
            };
            push_executable_candidates(&mut paths, directory, executable);
        }
        if executable == ManagedExecutable::Ripgrep
            && let Some(directory) = &self.executable_directory
        {
            push_executable_candidates(&mut paths, directory, executable);
        }
        if let Some(search_path) = &self.search_path {
            for directory in env::split_paths(search_path) {
                push_executable_candidates(&mut paths, &directory, executable);
            }
        }
        ExecutableCandidates::SearchPaths(paths)
    }

    fn detect(
        current_executable: Option<&Path>,
        ripgrep_override: Option<OsString>,
        bubblewrap_override: Option<OsString>,
        windows_command_runner_override: Option<OsString>,
        windows_sandbox_setup_override: Option<OsString>,
        search_path: Option<OsString>,
    ) -> Self {
        let executable_directory = current_executable
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        let package_layout = current_executable.and_then(detect_package_layout);
        let method = if package_layout.is_some() {
            InstallMethod::Package
        } else {
            InstallMethod::Other
        };
        Self {
            method,
            package_layout,
            executable_directory,
            ripgrep_override,
            bubblewrap_override,
            windows_command_runner_override,
            windows_sandbox_setup_override,
            search_path,
        }
    }
}

fn detect_package_layout(executable: &Path) -> Option<PackageLayout> {
    let binary_directory = executable.parent()?;
    if binary_directory.file_name()? != PACKAGE_BIN_DIRECTORY {
        return None;
    }
    let package_directory = binary_directory.parent()?;
    let metadata_file = package_directory.join(PACKAGE_METADATA_FILE);
    let path_directory = package_directory.join(PACKAGE_PATH_DIRECTORY);
    let resources_directory = package_directory.join(PACKAGE_RESOURCES_DIRECTORY);
    if !metadata_file.is_file() || !path_directory.is_dir() || !resources_directory.is_dir() {
        return None;
    }
    Some(PackageLayout {
        package_directory: package_directory.to_owned(),
        metadata_file,
        binary_directory: binary_directory.to_owned(),
        path_directory,
        resources_directory,
    })
}

fn is_safe_resource_name(name: &Path) -> bool {
    let mut components = name.components();
    components
        .next()
        .is_some_and(|component| matches!(component, Component::Normal(_)))
        && components.all(|component| matches!(component, Component::Normal(_)))
}

fn push_executable_candidates(
    candidates: &mut Vec<PathBuf>,
    directory: &Path,
    executable: ManagedExecutable,
) {
    for name in executable_names(executable) {
        let candidate = directory.join(name);
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
}

fn executable_names(executable: ManagedExecutable) -> &'static [&'static str] {
    match executable {
        ManagedExecutable::Ripgrep if cfg!(windows) => &["rg.exe", "rg"],
        ManagedExecutable::Ripgrep => &["rg"],
        ManagedExecutable::Bubblewrap => &["bwrap"],
        ManagedExecutable::WindowsCommandRunner => &["zeta-command-runner.exe"],
        ManagedExecutable::WindowsSandboxSetup => &["zeta-windows-sandbox-setup.exe"],
    }
}

#[cfg(test)]
#[path = "install_context_tests.rs"]
mod tests;
