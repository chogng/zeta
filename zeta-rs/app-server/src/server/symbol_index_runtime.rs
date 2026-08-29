use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use zeta_code_index::CodeIndex;
use zeta_symbol_index::SymbolIndex;
use zeta_symbol_index::SymbolIndexError;
use zeta_symbol_index::SymbolIndexLimits;
use zeta_symbol_index::SymbolIndexQuery;
use zeta_symbol_index::SymbolIndexRefreshOutcome;
use zeta_symbol_index::SymbolIndexSnapshot;
use zeta_symbol_index::SymbolIndexStorage;
use zeta_symbol_index::SymbolSearchHit;
use zeta_workspace_index_storage::WorkspaceIndexLease;

/// App Server-owned lifecycle projection for one workspace symbol index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SymbolIndexRuntimeState {
    Empty,
    Indexing {
        last_ready: Option<SymbolIndexSnapshot>,
    },
    Ready(SymbolIndexSnapshot),
    Stale(SymbolIndexSnapshot),
    Failed,
}

pub(super) struct SymbolIndexRuntime {
    source_index: Arc<CodeIndex>,
    index: Arc<SymbolIndex>,
    operation: Mutex<()>,
    state: RwLock<SymbolIndexRuntimeState>,
    _storage_lease: Option<WorkspaceIndexLease>,
}

impl SymbolIndexRuntime {
    pub fn open(
        source_index: Arc<CodeIndex>,
        storage: SymbolIndexStorage,
    ) -> Result<Arc<Self>, SymbolIndexError> {
        Self::open_inner(source_index, storage, None)
    }

    pub(super) fn open_with_lease(
        source_index: Arc<CodeIndex>,
        storage: SymbolIndexStorage,
        storage_lease: WorkspaceIndexLease,
    ) -> Result<Arc<Self>, SymbolIndexError> {
        Self::open_inner(source_index, storage, Some(storage_lease))
    }

    fn open_inner(
        source_index: Arc<CodeIndex>,
        storage: SymbolIndexStorage,
        storage_lease: Option<WorkspaceIndexLease>,
    ) -> Result<Arc<Self>, SymbolIndexError> {
        let index = Arc::new(SymbolIndex::open(
            Arc::clone(&source_index),
            storage,
            SymbolIndexLimits::default(),
        )?);
        let snapshot = index.snapshot()?;
        let state = if snapshot.generation == 0 {
            SymbolIndexRuntimeState::Empty
        } else {
            SymbolIndexRuntimeState::Stale(snapshot)
        };
        Ok(Arc::new(Self {
            source_index,
            index,
            operation: Mutex::new(()),
            state: RwLock::new(state),
            _storage_lease: storage_lease,
        }))
    }

    pub fn state(&self) -> SymbolIndexRuntimeState {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn root_id(&self) -> &zeta_code_index::IndexRootId {
        self.source_index.root_id()
    }

    pub fn index(&self) -> Arc<SymbolIndex> {
        Arc::clone(&self.index)
    }

    pub fn reconcile(&self) -> Result<SymbolIndexSnapshot, SymbolIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let last_ready = ready_snapshot(&self.state());
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = SymbolIndexRuntimeState::Indexing {
            last_ready: last_ready.clone(),
        };
        let result = self.index.reconcile().and_then(|outcome| match outcome {
            SymbolIndexRefreshOutcome::NoChange => self.index.snapshot(),
            SymbolIndexRefreshOutcome::Published(snapshot) => Ok(snapshot),
        });
        match result {
            Ok(snapshot) => {
                self.publish_ready(snapshot.clone());
                Ok(snapshot)
            }
            Err(error) => {
                let fallback = last_ready.map_or(
                    SymbolIndexRuntimeState::Failed,
                    SymbolIndexRuntimeState::Stale,
                );
                *self
                    .state
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = fallback;
                Err(error)
            }
        }
    }

    pub fn search(
        &self,
        query: &SymbolIndexQuery,
    ) -> Result<Vec<SymbolSearchHit>, SymbolIndexRuntimeError> {
        self.observe_source_generation()?;
        match self.state() {
            SymbolIndexRuntimeState::Ready(_) | SymbolIndexRuntimeState::Stale(_) => self
                .index
                .search(query)
                .map_err(SymbolIndexRuntimeError::Index),
            SymbolIndexRuntimeState::Indexing {
                last_ready: Some(_),
            } => self
                .index
                .search(query)
                .map_err(SymbolIndexRuntimeError::Index),
            SymbolIndexRuntimeState::Empty
            | SymbolIndexRuntimeState::Indexing { last_ready: None }
            | SymbolIndexRuntimeState::Failed => Err(SymbolIndexRuntimeError::NotReady),
        }
    }

    pub fn reconcile_overlay(&self) -> Result<(), SymbolIndexError> {
        self.index.reconcile_overlay()
    }

    fn observe_source_generation(&self) -> Result<(), SymbolIndexRuntimeError> {
        let source_generation = self
            .source_index
            .snapshot()
            .map_err(SymbolIndexRuntimeError::SourceIndex)?
            .generation;
        let symbol_snapshot = self
            .index
            .snapshot()
            .map_err(SymbolIndexRuntimeError::Index)?;
        if symbol_snapshot.generation > 0 && symbol_snapshot.source_generation != source_generation
        {
            *self
                .state
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                SymbolIndexRuntimeState::Stale(symbol_snapshot);
        }
        Ok(())
    }

    fn publish_ready(&self, snapshot: SymbolIndexSnapshot) {
        *self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) =
            SymbolIndexRuntimeState::Ready(snapshot);
    }
}

#[derive(Debug)]
pub(super) enum SymbolIndexRuntimeError {
    NotReady,
    SourceIndex(zeta_code_index::CodeIndexError),
    Index(SymbolIndexError),
}

fn ready_snapshot(state: &SymbolIndexRuntimeState) -> Option<SymbolIndexSnapshot> {
    match state {
        SymbolIndexRuntimeState::Ready(snapshot) | SymbolIndexRuntimeState::Stale(snapshot) => {
            Some(snapshot.clone())
        }
        SymbolIndexRuntimeState::Indexing { last_ready } => last_ready.clone(),
        SymbolIndexRuntimeState::Empty | SymbolIndexRuntimeState::Failed => None,
    }
}

#[cfg(test)]
#[path = "symbol_index_runtime_tests.rs"]
mod tests;
