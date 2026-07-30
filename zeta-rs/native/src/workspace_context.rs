use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceContext {
    working_directory: PathBuf,
    working_directory_label: String,
    git_branch: Option<String>,
    diff_count: Option<usize>,
}

impl WorkspaceContext {
    pub(crate) fn capture_current() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::capture(working_directory)
    }

    fn capture(working_directory: PathBuf) -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let working_directory_label =
            display_working_directory(&working_directory, home.as_deref());
        let (git_branch, diff_count) = repository_snapshot(&working_directory);
        Self {
            working_directory,
            working_directory_label,
            git_branch,
            diff_count,
        }
    }

    pub(crate) const fn location_label(&self) -> &'static str {
        "Local"
    }

    pub(crate) fn working_directory_label(&self) -> &str {
        &self.working_directory_label
    }

    pub(crate) fn git_branch_label(&self) -> &str {
        self.git_branch.as_deref().unwrap_or("No Git")
    }

    pub(crate) fn diff_count_label(&self) -> String {
        self.diff_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "—".to_string())
    }

    pub(crate) fn refresh_repository(&mut self) {
        (self.git_branch, self.diff_count) = repository_snapshot(&self.working_directory);
    }

    #[cfg(test)]
    pub(crate) fn fixture(
        working_directory_label: impl Into<String>,
        git_branch: Option<&str>,
        diff_count: Option<usize>,
    ) -> Self {
        Self {
            working_directory: PathBuf::from("/fixture"),
            working_directory_label: working_directory_label.into(),
            git_branch: git_branch.map(ToOwned::to_owned),
            diff_count,
        }
    }
}

fn display_working_directory(working_directory: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home {
        if working_directory == home {
            return "~".to_string();
        }
        if let Ok(relative) = working_directory.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    working_directory.display().to_string()
}

fn repository_snapshot(working_directory: &Path) -> (Option<String>, Option<usize>) {
    let branch = git_stdout(working_directory, &["branch", "--show-current"])
        .filter(|branch| !branch.is_empty())
        .or_else(|| {
            git_stdout(working_directory, &["rev-parse", "--short", "HEAD"])
                .filter(|commit| !commit.is_empty())
        });
    let diff_count = git_stdout(
        working_directory,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )
    .map(|status| status.lines().filter(|line| !line.is_empty()).count());
    (branch, diff_count)
}

fn git_stdout(working_directory: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(working_directory)
        .args(arguments)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
#[path = "workspace_context_tests.rs"]
mod tests;
