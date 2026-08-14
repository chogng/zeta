use std::path::{Path, PathBuf};
use std::{fs, io};

use zeta_app_server_protocol::protocol::git::{GitHeadDto, GitTextDiffResult};
use zeta_diff::DiffDocument;
use zeta_editor::{CodeEditorLanguage, DiffEditorDocument};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceDiff {
    path: String,
    document: DiffEditorDocument,
}

impl WorkspaceDiff {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) const fn document(&self) -> &DiffEditorDocument {
        &self.document
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceContext {
    working_directory: PathBuf,
    working_directory_label: String,
    location: WorkspaceLocation,
    git_branch: Option<String>,
    git_repository_root: Option<PathBuf>,
    upstream_distance: Option<(usize, usize)>,
    change_count: usize,
    diffs: Vec<WorkspaceDiff>,
    diff_additions: usize,
    diff_deletions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceLocation {
    Local,
    Remote,
}

impl WorkspaceContext {
    pub(crate) fn capture_current() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::capture_with_location(working_directory, WorkspaceLocation::Local)
    }

    #[cfg(test)]
    fn capture(working_directory: PathBuf) -> Self {
        Self::capture_with_location(working_directory, WorkspaceLocation::Local)
    }

    pub(crate) fn capture_remote(working_directory: PathBuf) -> Self {
        Self::capture_with_location(working_directory, WorkspaceLocation::Remote)
    }

    fn capture_with_location(working_directory: PathBuf, location: WorkspaceLocation) -> Self {
        let working_directory = match location {
            WorkspaceLocation::Local => working_directory
                .canonicalize()
                .unwrap_or(working_directory),
            WorkspaceLocation::Remote => working_directory,
        };
        let home = (location == WorkspaceLocation::Local)
            .then(|| {
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(PathBuf::from)
            })
            .flatten();
        let working_directory_label =
            display_working_directory(&working_directory, home.as_deref());
        Self {
            working_directory,
            working_directory_label,
            location,
            git_branch: None,
            git_repository_root: None,
            upstream_distance: None,
            change_count: 0,
            diffs: Vec::new(),
            diff_additions: 0,
            diff_deletions: 0,
        }
    }

    pub(crate) const fn location_label(&self) -> &'static str {
        match self.location {
            WorkspaceLocation::Local => "Local",
            WorkspaceLocation::Remote => "Remote",
        }
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

    pub(crate) fn git_repository_root(&self) -> Option<&Path> {
        self.git_repository_root.as_deref()
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

    pub(crate) fn apply_git_projection(&mut self, projection: Option<&GitTextDiffResult>) {
        self.clear_repository();
        let Some(projection) = projection else {
            return;
        };
        let (branch, upstream_distance) = match &projection.status.head {
            GitHeadDto::Branch { name, upstream, .. } => (
                name.clone(),
                upstream
                    .as_ref()
                    .map(|upstream| (upstream.ahead, upstream.behind)),
            ),
            GitHeadDto::Detached { object_id } => {
                (object_id.chars().take(8).collect::<String>(), None)
            }
            GitHeadDto::Unborn { name } => (name.clone(), None),
        };
        self.git_branch = Some(branch);
        self.git_repository_root = repository_root_from_workspace_path(
            &self.working_directory,
            &projection.status.workspace_path,
        );
        self.upstream_distance = upstream_distance;
        self.change_count = projection.status.changes.len();
        self.diffs = projection
            .diffs
            .iter()
            .filter_map(|diff| {
                DiffDocument::from_text(&diff.original, &diff.modified)
                    .ok()
                    .map(|document| WorkspaceDiff {
                        path: diff.path.clone(),
                        document: DiffEditorDocument::new(
                            document,
                            editor_language_for_path(Path::new(&diff.path)),
                        ),
                    })
            })
            .collect();
        self.diff_additions = projection.statistics.additions;
        self.diff_deletions = projection.statistics.deletions;
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
        *self = Self::capture_with_location(working_directory, WorkspaceLocation::Local);
        Ok(())
    }

    fn clear_repository(&mut self) {
        self.git_branch = None;
        self.git_repository_root = None;
        self.upstream_distance = None;
        self.change_count = 0;
        self.diffs.clear();
        self.diff_additions = 0;
        self.diff_deletions = 0;
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
                        document: DiffEditorDocument::new(document, CodeEditorLanguage::PlainText),
                    })
            })
            .collect();
        Self {
            working_directory: PathBuf::from("/fixture"),
            working_directory_label: working_directory_label.into(),
            location: WorkspaceLocation::Local,
            git_branch: git_branch.map(ToOwned::to_owned),
            git_repository_root: git_branch.map(|_| PathBuf::from("/fixture")),
            upstream_distance: git_branch.map(|_| (0, 0)),
            change_count: diffs.len(),
            diff_additions: diffs.len(),
            diff_deletions: 0,
            diffs,
        }
    }
}

fn repository_root_from_workspace_path(
    working_directory: &Path,
    workspace_path: &str,
) -> Option<PathBuf> {
    let workspace_path = Path::new(workspace_path);
    if workspace_path.as_os_str().is_empty() {
        return Some(working_directory.to_path_buf());
    }
    let components = workspace_path.components().collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }
    let repository_root = working_directory.ancestors().nth(components.len())?;
    let actual_suffix = working_directory.strip_prefix(repository_root).ok()?;
    actual_suffix
        .components()
        .eq(components)
        .then(|| repository_root.to_path_buf())
}

fn editor_language_for_path(path: &Path) -> CodeEditorLanguage {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => CodeEditorLanguage::Rust,
        Some("json") => CodeEditorLanguage::Json,
        Some("jsonc") => CodeEditorLanguage::Jsonc,
        Some("sh" | "bash" | "zsh") => CodeEditorLanguage::Shell,
        _ => CodeEditorLanguage::PlainText,
    }
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
