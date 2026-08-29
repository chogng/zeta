use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

const BINDING_FILENAME: &str = "zeta-thread-workspace.json";
const BINDING_VERSION: u8 = 4;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryBindingRecord {
    pub repository_id: String,
    pub relative_path: PathBuf,
    pub source_repository_root: PathBuf,
    pub target_branch: Option<String>,
    pub target_head: String,
    #[serde(default)]
    pub target_unborn: bool,
    pub baseline_tree: String,
    pub baseline_ref: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum BindingKind {
    #[default]
    Git,
    Directory,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BindingRecord {
    version: u8,
    pub managed_worktree_id: String,
    pub owner_thread_id: String,
    pub source_workspace_id: String,
    pub source_repository_root: PathBuf,
    pub relative_workspace_directory: PathBuf,
    pub target_branch: Option<String>,
    pub target_head: String,
    #[serde(default)]
    pub target_unborn: bool,
    pub baseline_tree: String,
    pub baseline_ref: String,
    #[serde(default)]
    pub kind: BindingKind,
    #[serde(default)]
    pub snapshot_store: Option<PathBuf>,
    #[serde(default)]
    pub repositories: Vec<RepositoryBindingRecord>,
}

impl BindingRecord {
    pub(crate) fn new(
        managed_worktree_id: String,
        owner_thread_id: String,
        source_workspace_id: String,
        source_repository_root: PathBuf,
        relative_workspace_directory: PathBuf,
        target_branch: Option<String>,
        target_head: String,
        target_unborn: bool,
        baseline_tree: String,
        baseline_ref: String,
        kind: BindingKind,
        snapshot_store: Option<PathBuf>,
    ) -> Self {
        Self {
            version: BINDING_VERSION,
            managed_worktree_id,
            owner_thread_id,
            source_workspace_id,
            source_repository_root,
            relative_workspace_directory,
            target_branch,
            target_head,
            target_unborn,
            baseline_tree,
            baseline_ref,
            kind,
            snapshot_store,
            repositories: Vec::new(),
        }
    }

    pub(crate) fn with_repositories(mut self, repositories: Vec<RepositoryBindingRecord>) -> Self {
        self.repositories = repositories;
        self
    }
}

pub(crate) fn read(git_dir: &Path) -> Result<BindingRecord> {
    let path = git_dir.join(BINDING_FILENAME);
    let record = serde_json::from_slice::<BindingRecord>(
        &fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?,
    )
    .with_context(|| format!("invalid Thread workspace binding at {}", path.display()))?;
    if !(1..=BINDING_VERSION).contains(&record.version)
        || record.managed_worktree_id.is_empty()
        || record.owner_thread_id.is_empty()
        || record.target_head.is_empty()
        || record.baseline_tree.is_empty()
        || (record.kind == BindingKind::Git && record.baseline_ref.is_empty())
        || (record.kind == BindingKind::Directory && record.snapshot_store.is_none())
    {
        bail!("invalid Thread workspace binding at {}", path.display());
    }
    Ok(record)
}

pub(crate) fn try_read(directory: &Path) -> Result<Option<BindingRecord>> {
    if !directory.join(BINDING_FILENAME).exists() {
        return Ok(None);
    }
    read(directory).map(Some)
}

pub(crate) fn write(git_dir: &Path, record: &BindingRecord) -> Result<()> {
    let path = git_dir.join(BINDING_FILENAME);
    if path.exists() {
        let current = read(git_dir)?;
        if &current == record {
            return Ok(());
        }
        bail!(
            "Thread workspace binding already exists at {}",
            path.display()
        );
    }
    let mut temporary = NamedTempFile::new_in(git_dir).with_context(|| {
        format!(
            "cannot create Thread workspace binding in {}",
            git_dir.display()
        )
    })?;
    serde_json::to_writer(&mut temporary, record)?;
    temporary.flush()?;
    temporary
        .persist_noclobber(&path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| {
            format!(
                "cannot write Thread workspace binding at {}",
                path.display()
            )
        })
}

pub(crate) fn replace(git_dir: &Path, record: &BindingRecord) -> Result<()> {
    let current = read(git_dir)?;
    if current.owner_thread_id != record.owner_thread_id
        || current.managed_worktree_id != record.managed_worktree_id
    {
        bail!("Thread workspace binding owner changed during update");
    }
    let path = git_dir.join(BINDING_FILENAME);
    let mut temporary = NamedTempFile::new_in(git_dir)?;
    serde_json::to_writer(&mut temporary, record)?;
    temporary.flush()?;
    temporary
        .persist(&path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("cannot replace {}", path.display()))
}
