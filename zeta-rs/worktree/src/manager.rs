use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use zeta_git::GitClient;
use zeta_git::GitRepository;

use crate::metadata;
use crate::settings::WorktreeSettings;

pub use zeta_git::GitWorktreeAvailability as WorktreeAvailability;

/// Physical role of one checkout in a repository's worktree inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeKind {
    Primary,
    Linked,
}

/// Explicit selector used to resolve one worktree switch target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeSelector {
    Branch(String),
    CheckoutRoot(PathBuf),
}

/// State of Codex thread ownership metadata attached to one worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorktreeOwner {
    Unbound,
    Thread(String),
    Invalid,
}

/// One Git worktree and the workspace directory corresponding to the caller's source directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Worktree {
    checkout_root: PathBuf,
    workspace_directory: PathBuf,
    head: String,
    branch: Option<String>,
    kind: WorktreeKind,
    availability: WorktreeAvailability,
    current: bool,
    owner: WorktreeOwner,
}

impl Worktree {
    pub fn checkout_root(&self) -> &Path {
        &self.checkout_root
    }

    pub fn workspace_directory(&self) -> &Path {
        &self.workspace_directory
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub const fn kind(&self) -> WorktreeKind {
        self.kind
    }

    pub const fn availability(&self) -> &WorktreeAvailability {
        &self.availability
    }

    pub const fn is_current(&self) -> bool {
        self.current
    }

    pub fn owner_thread_id(&self) -> Option<&str> {
        match &self.owner {
            WorktreeOwner::Thread(thread_id) => Some(thread_id),
            WorktreeOwner::Unbound | WorktreeOwner::Invalid => None,
        }
    }

    pub const fn owner(&self) -> &WorktreeOwner {
        &self.owner
    }
}

/// Discovers switch targets and manages Codex-compatible ownership for managed worktrees.
#[derive(Clone, Debug)]
pub struct WorktreeManager {
    settings: WorktreeSettings,
    git: GitClient,
}

impl WorktreeManager {
    pub fn new(mut settings: WorktreeSettings) -> Self {
        settings.root = dunce::simplified(&settings.root).to_path_buf();
        Self {
            settings,
            git: GitClient::system(),
        }
    }

    pub fn settings(&self) -> &WorktreeSettings {
        &self.settings
    }

    /// Lists the repository's worktrees while preserving the source directory's relative cwd.
    pub async fn list(&self, source_directory: &Path) -> Result<Vec<Worktree>> {
        let source_directory = dunce::canonicalize(source_directory).with_context(|| {
            format!(
                "cannot resolve source directory {}",
                source_directory.display()
            )
        })?;
        let source_repository = self.git.open_repository(&source_directory).await?;
        let relative_directory = source_directory
            .strip_prefix(source_repository.worktree_root())
            .context("source directory is outside its repository root")?;
        let entries = self.git.worktrees(&source_repository).await?;
        let mut worktrees = Vec::with_capacity(entries.len());
        for (index, entry) in entries.into_iter().enumerate() {
            let current = if entry.checkout_root() == source_repository.worktree_root() {
                true
            } else if entry.checkout_root().exists() {
                existing_paths_match(entry.checkout_root(), source_repository.worktree_root())?
            } else {
                false
            };
            let owner = if entry.availability().is_available() && entry.checkout_root().exists() {
                let repository = self.git.open_repository(entry.checkout_root()).await?;
                match metadata::owner(repository.git_dir()) {
                    Ok(Some(thread_id)) => WorktreeOwner::Thread(thread_id),
                    Ok(None) => WorktreeOwner::Unbound,
                    Err(_) => WorktreeOwner::Invalid,
                }
            } else {
                WorktreeOwner::Unbound
            };
            worktrees.push(Worktree {
                workspace_directory: entry.checkout_root().join(relative_directory),
                checkout_root: entry.checkout_root().to_path_buf(),
                head: entry.head().to_string(),
                branch: entry.branch().map(ToOwned::to_owned),
                kind: if index == 0 {
                    WorktreeKind::Primary
                } else {
                    WorktreeKind::Linked
                },
                availability: entry.availability().clone(),
                current,
                owner,
            });
        }
        Ok(worktrees)
    }

    /// Resolves one available worktree without changing process or product state.
    pub async fn resolve(
        &self,
        source_directory: &Path,
        selector: &WorktreeSelector,
    ) -> Result<Worktree> {
        let checkout_root = match selector {
            WorktreeSelector::Branch(branch) => {
                if branch.trim().is_empty() {
                    bail!("worktree branch selector cannot be empty");
                }
                None
            }
            WorktreeSelector::CheckoutRoot(root) => {
                if !root.is_absolute() {
                    bail!("worktree checkout selector must be an absolute path");
                }
                Some(dunce::canonicalize(root).with_context(|| {
                    format!("cannot resolve worktree checkout {}", root.display())
                })?)
            }
        };
        let worktrees = self.list(source_directory).await?;
        let mut matching = Vec::new();
        for worktree in worktrees {
            let matches = match selector {
                WorktreeSelector::Branch(branch) => worktree.branch() == Some(branch.as_str()),
                WorktreeSelector::CheckoutRoot(_) if !worktree.checkout_root().exists() => false,
                WorktreeSelector::CheckoutRoot(_) => existing_paths_match(
                    worktree.checkout_root(),
                    checkout_root
                        .as_deref()
                        .expect("checkout selector was canonicalized"),
                )?,
            };
            if matches {
                matching.push(worktree);
            }
        }
        if matching.len() > 1 {
            bail!("worktree selector matched more than one checkout");
        }
        let worktree = matching
            .pop()
            .context("worktree selector did not match a checkout")?;
        if !worktree.availability().is_available() {
            bail!(
                "worktree {} is not available",
                worktree.checkout_root().display()
            );
        }
        if !worktree.workspace_directory().is_dir() {
            bail!(
                "worktree workspace directory does not exist: {}",
                worktree.workspace_directory().display()
            );
        }
        Ok(worktree)
    }

    pub async fn bind_thread(&self, checkout: &Path, thread_id: &str) -> Result<()> {
        let repository = self.managed_checkout(checkout).await?;
        metadata::bind_thread(repository.git_dir(), thread_id)
    }

    pub async fn owner(&self, checkout: &Path) -> Result<Option<String>> {
        let repository = self.managed_checkout(checkout).await?;
        metadata::owner(repository.git_dir())
    }

    async fn managed_checkout(&self, checkout: &Path) -> Result<GitRepository> {
        let managed_root = dunce::canonicalize(&self.settings.root).with_context(|| {
            format!(
                "cannot resolve managed worktree root {}",
                self.settings.root.display()
            )
        })?;
        let checkout = dunce::canonicalize(checkout)
            .with_context(|| format!("cannot resolve worktree {}", checkout.display()))?;
        if !has_managed_layout(&managed_root, &checkout) {
            bail!("{} is not a managed worktree", checkout.display());
        }
        let repository = self.git.open_repository(&checkout).await?;
        if repository.worktree_root() != checkout {
            bail!("{} is not a worktree root", checkout.display());
        }
        if repository.git_dir() == repository.common_dir() {
            bail!("{} is not a linked worktree", checkout.display());
        }
        Ok(repository)
    }
}

fn has_managed_layout(root: &Path, checkout: &Path) -> bool {
    let Ok(relative) = checkout.strip_prefix(root) else {
        return false;
    };
    let mut components = relative.components();
    let Some(Component::Normal(bucket)) = components.next() else {
        return false;
    };
    let Some(bucket) = bucket.to_str() else {
        return false;
    };
    bucket.len() == 4
        && bucket.bytes().all(|byte| byte.is_ascii_hexdigit())
        && matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
}

fn existing_paths_match(left: &Path, right: &Path) -> Result<bool> {
    Ok(dunce::canonicalize(left)? == dunce::canonicalize(right)?)
}
