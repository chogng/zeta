use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{Context, Result};
use zeta_diff::DiffDocument;
use zeta_git::{GitBranch, GitClient, GitHead, GitTextDiffLimits};

const MAX_DIFF_FILE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceDiff {
    path: String,
    document: DiffDocument,
}

impl WorkspaceDiff {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) const fn document(&self) -> &DiffDocument {
        &self.document
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceContext {
    working_directory: PathBuf,
    working_directory_label: String,
    git_branch: Option<String>,
    upstream_distance: Option<(usize, usize)>,
    change_count: usize,
    diffs: Vec<WorkspaceDiff>,
    diff_additions: usize,
    diff_deletions: usize,
}

impl WorkspaceContext {
    pub(crate) fn capture_current() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::capture(working_directory)
    }

    fn capture(working_directory: PathBuf) -> Self {
        let working_directory = working_directory
            .canonicalize()
            .unwrap_or(working_directory);
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let working_directory_label =
            display_working_directory(&working_directory, home.as_deref());
        let mut context = Self {
            working_directory,
            working_directory_label,
            git_branch: None,
            upstream_distance: None,
            change_count: 0,
            diffs: Vec::new(),
            diff_additions: 0,
            diff_deletions: 0,
        };
        context.refresh_repository();
        context
    }

    pub(crate) const fn location_label(&self) -> &'static str {
        "Local"
    }

    pub(crate) fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub(crate) fn working_directory_label(&self) -> &str {
        &self.working_directory_label
    }

    pub(crate) fn git_branch_label(&self) -> &str {
        self.git_branch.as_deref().unwrap_or("No Git")
    }

    pub(crate) fn diff_summary_label(&self) -> String {
        self.git_branch
            .as_ref()
            .map(|_| {
                format!(
                    "Changes {} • +{} -{}",
                    self.change_count, self.diff_additions, self.diff_deletions
                )
            })
            .unwrap_or_else(|| "Changes —".to_string())
    }

    pub(crate) const fn upstream_distance(&self) -> Option<(usize, usize)> {
        self.upstream_distance
    }

    pub(crate) fn diffs(&self) -> &[WorkspaceDiff] {
        &self.diffs
    }

    pub(crate) fn refresh_repository(&mut self) {
        let Some(snapshot) = repository_snapshot(&self.working_directory) else {
            self.git_branch = None;
            self.upstream_distance = None;
            self.change_count = 0;
            self.diffs.clear();
            self.diff_additions = 0;
            self.diff_deletions = 0;
            return;
        };
        self.git_branch = Some(snapshot.branch);
        self.upstream_distance = snapshot.upstream_distance;
        self.change_count = snapshot.change_count;
        self.diffs = snapshot.diffs;
        self.diff_additions = snapshot.diff_additions;
        self.diff_deletions = snapshot.diff_deletions;
    }

    pub(crate) fn switch_working_directory(
        &mut self,
        working_directory: PathBuf,
    ) -> io::Result<()> {
        let working_directory = working_directory.canonicalize()?;
        if !fs::metadata(&working_directory)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a directory", working_directory.display()),
            ));
        }
        *self = Self::capture(working_directory);
        Ok(())
    }

    pub(crate) fn local_branches(&self) -> Result<Vec<GitBranch>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("could not create Git branch query runtime")?;
        runtime.block_on(async {
            let client = GitClient::system();
            let repository = client
                .open_repository(&self.working_directory)
                .await
                .context("could not open workspace Git repository")?;
            client
                .local_branches(&repository)
                .await
                .context("could not list local Git branches")
        })
    }

    pub(crate) fn switch_branch(&mut self, branch: &GitBranch) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("could not create Git branch mutation runtime")?;
        runtime.block_on(async {
            let client = GitClient::system();
            let repository = client
                .open_repository(&self.working_directory)
                .await
                .context("could not open workspace Git repository")?;
            client
                .switch_branch(&repository, branch)
                .await
                .with_context(|| format!("could not switch to Git branch {}", branch.name()))
        })?;
        self.refresh_repository();
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fixture(
        working_directory_label: impl Into<String>,
        git_branch: Option<&str>,
        diff_count: Option<usize>,
    ) -> Self {
        let diffs: Vec<_> = (0..diff_count.unwrap_or(0))
            .filter_map(|index| {
                DiffDocument::from_text("", &format!("fixture {index}\n"))
                    .ok()
                    .map(|document| WorkspaceDiff {
                        path: format!("fixture-{index}.txt"),
                        document,
                    })
            })
            .collect();
        Self {
            working_directory: PathBuf::from("/fixture"),
            working_directory_label: working_directory_label.into(),
            git_branch: git_branch.map(ToOwned::to_owned),
            upstream_distance: git_branch.map(|_| (0, 0)),
            change_count: diffs.len(),
            diff_additions: diffs.len(),
            diff_deletions: 0,
            diffs,
        }
    }
}

struct RepositorySnapshot {
    branch: String,
    upstream_distance: Option<(usize, usize)>,
    change_count: usize,
    diffs: Vec<WorkspaceDiff>,
    diff_additions: usize,
    diff_deletions: usize,
}

fn repository_snapshot(working_directory: &Path) -> Option<RepositorySnapshot> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let client = GitClient::system();
        let repository = client.open_repository(working_directory).await.ok()?;
        let workspace_prefix = working_directory
            .strip_prefix(repository.worktree_root())
            .ok()?;
        let diff_snapshot = client
            .text_diff_snapshot_under(
                &repository,
                workspace_prefix,
                GitTextDiffLimits::new(MAX_DIFF_FILE_BYTES).ok()?,
            )
            .await
            .ok()?;
        let snapshot = diff_snapshot.repository();
        let (branch, upstream_distance) = match snapshot.head() {
            GitHead::Branch { name, upstream, .. } => (
                name.clone(),
                upstream
                    .as_ref()
                    .map(|upstream| (upstream.ahead(), upstream.behind())),
            ),
            GitHead::Detached { object_id } => (object_id.chars().take(8).collect(), None),
            GitHead::Unborn { name } => (name.clone(), None),
        };
        let change_count = snapshot
            .changes()
            .iter()
            .filter(|change| change.path().strip_prefix(workspace_prefix).is_ok())
            .count();
        let mut diffs = Vec::new();
        let mut diff_additions = 0;
        let mut diff_deletions = 0;
        for diff in diff_snapshot.diffs() {
            let Some(display_path) = diff.path().strip_prefix(workspace_prefix).ok() else {
                continue;
            };
            let statistics = diff.statistics();
            diff_additions += statistics.additions();
            diff_deletions += statistics.deletions();
            diffs.push(WorkspaceDiff {
                path: display_path.to_string_lossy().replace('\\', "/"),
                document: diff.document().clone(),
            });
        }
        Some(RepositorySnapshot {
            branch,
            upstream_distance,
            change_count,
            diffs,
            diff_additions,
            diff_deletions,
        })
    })
}

pub(crate) fn display_working_directory(working_directory: &Path, home: Option<&Path>) -> String {
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

#[cfg(test)]
#[path = "workspace_context_tests.rs"]
mod tests;
