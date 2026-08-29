use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexError;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexQuery;
use zeta_code_index::CodeIndexSnapshot;
use zeta_code_index::CodeIndexStorage;
use zeta_code_index::RefreshOutcome;
use zeta_code_index::SearchHit;
use zeta_file_watcher::FileWatcherEvent;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace_index_storage::WorkspaceIndexLease;

/// App Server-owned lifecycle projection for one workspace-side code index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CodeIndexRuntimeState {
    Empty,
    Indexing {
        last_ready: Option<CodeIndexSnapshot>,
    },
    Ready(CodeIndexSnapshot),
    Stale(CodeIndexSnapshot),
    Failed,
}

pub(super) struct CodeIndexRuntime {
    index: Arc<CodeIndex>,
    operation: Mutex<()>,
    state: RwLock<CodeIndexRuntimeState>,
    _storage_lease: Option<WorkspaceIndexLease>,
}

impl CodeIndexRuntime {
    pub fn open(
        workspace: WorkspaceRoot,
        storage: CodeIndexStorage,
    ) -> Result<Arc<Self>, CodeIndexError> {
        Self::open_inner(workspace, storage, None)
    }

    pub(super) fn open_with_lease(
        workspace: WorkspaceRoot,
        storage: CodeIndexStorage,
        storage_lease: WorkspaceIndexLease,
    ) -> Result<Arc<Self>, CodeIndexError> {
        Self::open_inner(workspace, storage, Some(storage_lease))
    }

    fn open_inner(
        workspace: WorkspaceRoot,
        storage: CodeIndexStorage,
        storage_lease: Option<WorkspaceIndexLease>,
    ) -> Result<Arc<Self>, CodeIndexError> {
        let index = Arc::new(CodeIndex::open(
            workspace,
            storage,
            CodeIndexLimits::default(),
        )?);
        let snapshot = index.snapshot()?;
        let state = if snapshot.generation == 0 {
            CodeIndexRuntimeState::Empty
        } else {
            CodeIndexRuntimeState::Stale(snapshot)
        };
        Ok(Arc::new(Self {
            index,
            operation: Mutex::new(()),
            state: RwLock::new(state),
            _storage_lease: storage_lease,
        }))
    }

    pub fn root(&self) -> &WorkspaceRoot {
        self.index.root()
    }

    pub fn index(&self) -> Arc<CodeIndex> {
        Arc::clone(&self.index)
    }

    pub fn state(&self) -> CodeIndexRuntimeState {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn rebuild(&self) -> Result<CodeIndexSnapshot, CodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let last_ready = ready_snapshot(&self.state());
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = CodeIndexRuntimeState::Indexing {
            last_ready: last_ready.clone(),
        };
        match self.index.rebuild() {
            Ok(snapshot) => {
                *self
                    .state
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    CodeIndexRuntimeState::Ready(snapshot.clone());
                Ok(snapshot)
            }
            Err(error) => {
                let fallback =
                    last_ready.map_or(CodeIndexRuntimeState::Failed, CodeIndexRuntimeState::Stale);
                *self
                    .state
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = fallback;
                Err(error)
            }
        }
    }

    pub fn apply_watcher_event(&self, event: &FileWatcherEvent) {
        let result = match event {
            FileWatcherEvent::PathsChanged { paths } => self.refresh_paths(paths),
            FileWatcherEvent::RescanRequired { .. } => self.rebuild().map(|_| ()),
        };
        if let Err(error) = result {
            log::warn!("code-index refresh failed: {error}");
        }
    }

    pub fn search(&self, query: &CodeIndexQuery) -> Result<Vec<SearchHit>, CodeIndexRuntimeError> {
        self.ensure_searchable()?;
        self.search_ready(query)
    }

    pub fn ensure_searchable(&self) -> Result<(), CodeIndexRuntimeError> {
        match self.state() {
            CodeIndexRuntimeState::Ready(_) | CodeIndexRuntimeState::Stale(_) => Ok(()),
            CodeIndexRuntimeState::Empty
            | CodeIndexRuntimeState::Indexing { last_ready: None }
            | CodeIndexRuntimeState::Failed => Err(CodeIndexRuntimeError::NotReady),
            CodeIndexRuntimeState::Indexing {
                last_ready: Some(_),
            } => Ok(()),
        }
    }

    fn search_ready(
        &self,
        query: &CodeIndexQuery,
    ) -> Result<Vec<SearchHit>, CodeIndexRuntimeError> {
        let hits = self
            .index
            .search(query)
            .map_err(CodeIndexRuntimeError::Index)?;
        hits.into_iter()
            .map(|mut hit| {
                let materialized = self.index.materialize(&hit.reference).map_err(|error| {
                    if let Some(snapshot) = ready_snapshot(&self.state()) {
                        *self
                            .state
                            .write()
                            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                            CodeIndexRuntimeState::Stale(snapshot);
                    }
                    CodeIndexRuntimeError::Index(error)
                })?;
                hit.content = materialized.content;
                Ok(hit)
            })
            .collect()
    }

    fn refresh_paths(&self, paths: &[PathBuf]) -> Result<(), CodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_state = self.state();
        let last_ready = ready_snapshot(&previous_state);
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = CodeIndexRuntimeState::Indexing {
            last_ready: last_ready.clone(),
        };
        match self.index.refresh_observed_paths(paths) {
            Ok(RefreshOutcome::NoChange) => {
                let restored = match self.state() {
                    CodeIndexRuntimeState::Stale(snapshot) => {
                        CodeIndexRuntimeState::Stale(snapshot)
                    }
                    _ => previous_state,
                };
                *self
                    .state
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = restored;
                Ok(())
            }
            Ok(RefreshOutcome::Published(snapshot) | RefreshOutcome::Rebuilt(snapshot)) => {
                *self
                    .state
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    CodeIndexRuntimeState::Ready(snapshot);
                Ok(())
            }
            Err(error) => {
                if let Some(snapshot) = last_ready {
                    *self
                        .state
                        .write()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        CodeIndexRuntimeState::Stale(snapshot);
                } else {
                    *self
                        .state
                        .write()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                        CodeIndexRuntimeState::Failed;
                }
                Err(error)
            }
        }
    }
}

#[derive(Debug)]
pub(super) enum CodeIndexRuntimeError {
    NotReady,
    Index(CodeIndexError),
}

fn ready_snapshot(state: &CodeIndexRuntimeState) -> Option<CodeIndexSnapshot> {
    match state {
        CodeIndexRuntimeState::Ready(snapshot) | CodeIndexRuntimeState::Stale(snapshot) => {
            Some(snapshot.clone())
        }
        CodeIndexRuntimeState::Indexing { last_ready } => last_ready.clone(),
        CodeIndexRuntimeState::Empty | CodeIndexRuntimeState::Failed => None,
    }
}

#[cfg(test)]
#[path = "code_index_runtime_tests.rs"]
mod tests;
