use std::path::{Path, PathBuf};
use std::process::Command;
use zeta_agent_environment::AgentEnvironmentError;
use zeta_agent_environment::AgentEnvironmentSnapshot;
use zeta_agent_environment::HostEnvironment;
use zeta_agent_environment::RepositoryEnvironment;
use zeta_agent_environment::WorkspaceRoots;

const MAX_GIT_STATUS_LINES: usize = 40;

/// App Server-owned collection of immutable host and repository facts for one Workspace.
#[derive(Clone)]
pub(super) struct WorkspaceEnvironment {
    host: HostEnvironment,
    repository: RepositoryEnvironment,
}

impl WorkspaceEnvironment {
    pub(super) fn capture(workspace_root: &Path) -> Result<Self, AgentEnvironmentError> {
        let is_git_repo = command_output(
            workspace_root,
            "git",
            &["rev-parse", "--is-inside-work-tree"],
        )
        .is_some_and(|output| output.trim() == "true");
        let branch = is_git_repo
            .then(|| command_output(workspace_root, "git", &["branch", "--show-current"]))
            .flatten()
            .filter(|value| !value.trim().is_empty());
        let main_branch = is_git_repo
            .then(|| {
                command_output(
                    workspace_root,
                    "git",
                    &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
                )
            })
            .flatten()
            .map(|value| value.trim_start_matches("origin/").trim().to_owned())
            .filter(|value| !value.is_empty())
            .or_else(|| branch.clone());
        let repository = if is_git_repo {
            let status = command_output(workspace_root, "git", &["status", "--porcelain"])
                .map(|value| truncate_lines(&value, MAX_GIT_STATUS_LINES))
                .unwrap_or_else(|| "(none)".into());
            let recent_commits = command_output(workspace_root, "git", &["log", "--oneline", "-5"])
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "(none)".into());
            RepositoryEnvironment::git(branch, main_branch, status, recent_commits)?
        } else {
            RepositoryEnvironment::NotDetected
        };
        let host = HostEnvironment::new(
            workspace_root.to_path_buf(),
            platform_name().into(),
            command_output(workspace_root, "uname", &["-sr"])
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".into()),
            std::env::var("SHELL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".into()),
            command_output(workspace_root, "date", &["+%Y-%m-%d"])
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".into()),
        )?;
        Ok(Self { host, repository })
    }

    pub(super) fn snapshot(
        &self,
        additional_roots: impl IntoIterator<Item = PathBuf>,
    ) -> Result<AgentEnvironmentSnapshot, AgentEnvironmentError> {
        let workspace_roots = WorkspaceRoots::new(self.host.cwd().to_path_buf(), additional_roots)?;
        Ok(AgentEnvironmentSnapshot::new(
            self.host.clone(),
            self.repository.clone(),
            workspace_roots,
        ))
    }
}

fn truncate_lines(text: &str, maximum_lines: usize) -> String {
    let mut lines = text.lines().take(maximum_lines).collect::<Vec<_>>();
    if text.lines().count() > maximum_lines {
        lines.push("[... git status truncated ...]");
    }
    if lines.is_empty() {
        "(clean)".into()
    } else {
        lines.join("\n")
    }
}

fn command_output(workspace_root: &Path, program: &str, arguments: &[&str]) -> Option<String> {
    let output = if program == "git" {
        Command::new(program)
            .args(["-C", &workspace_root.to_string_lossy()])
            .args(arguments)
            .output()
    } else {
        Command::new(program)
            .args(arguments)
            .current_dir(workspace_root)
            .output()
    }
    .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}
