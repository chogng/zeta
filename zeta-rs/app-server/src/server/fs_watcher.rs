use super::code_index_runtime::CodeIndexRuntime;
use super::semantic_index_job::SemanticIndexJobController;
use super::update_broker::UpdateBroker;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_file_watcher::DebouncedWatchReceiver;
use zeta_file_watcher::FileWatcher;
use zeta_file_watcher::FileWatcherBackend;
use zeta_file_watcher::FileWatcherEvent;
use zeta_file_watcher::WatchPath;
use zeta_workspace::WorkspaceRoot;

const FILE_SYSTEM_WATCH_DEBOUNCE: Duration = Duration::from_millis(75);
const ALIASED_PATH_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Receives projected Workspace filesystem invalidations before client publication.
///
/// Implementations must keep the callback bounded because it runs on the watcher thread.
pub(crate) trait WorkspaceFileChangeSink: Send + Sync {
    fn files_changed(&self, changed: &FsChanged);
}

enum FileSystemWatcherObservers {
    None,
    WorkspaceRuntime {
        code_index: Arc<CodeIndexRuntime>,
        code_index_semantic: Option<Arc<SemanticIndexJobController>>,
        changes: Arc<dyn WorkspaceFileChangeSink>,
    },
}

#[derive(Default)]
enum PendingCodeIndexRefresh {
    #[default]
    None,
    Paths(BTreeSet<PathBuf>),
    Rebuild,
}

impl PendingCodeIndexRefresh {
    fn merge(&mut self, event: FileWatcherEvent) {
        match event {
            FileWatcherEvent::RescanRequired { .. } => *self = Self::Rebuild,
            FileWatcherEvent::PathsChanged { paths } => match self {
                Self::None => *self = Self::Paths(paths.into_iter().collect()),
                Self::Paths(pending) => pending.extend(paths),
                Self::Rebuild => {}
            },
        }
    }

    fn take_event(&mut self) -> Option<FileWatcherEvent> {
        match std::mem::take(self) {
            Self::None => None,
            Self::Paths(paths) => Some(FileWatcherEvent::PathsChanged {
                paths: paths.into_iter().collect(),
            }),
            Self::Rebuild => Some(FileWatcherEvent::RescanRequired {
                watched_paths: Vec::new(),
            }),
        }
    }
}

struct CodeIndexRefreshWorker {
    pending: Arc<Mutex<PendingCodeIndexRefresh>>,
    wake: Option<SyncSender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl CodeIndexRefreshWorker {
    fn start(
        runtime: Arc<CodeIndexRuntime>,
        semantic: Option<Arc<SemanticIndexJobController>>,
    ) -> Result<Self, String> {
        let pending = Arc::new(Mutex::new(PendingCodeIndexRefresh::None));
        let (wake, receiver) = std::sync::mpsc::sync_channel(1);
        let worker_runtime = Arc::clone(&runtime);
        let worker_pending = Arc::clone(&pending);
        let thread = std::thread::Builder::new()
            .name("zeta-code-index-refresh".into())
            .spawn(move || {
                while receiver.recv().is_ok() {
                    let event = worker_pending
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .take_event();
                    if let Some(event) = event {
                        let previous_generation = worker_runtime
                            .index()
                            .snapshot()
                            .map(|snapshot| snapshot.generation)
                            .ok();
                        worker_runtime.apply_watcher_event(&event);
                        let current_generation = worker_runtime
                            .index()
                            .snapshot()
                            .map(|snapshot| snapshot.generation)
                            .ok();
                        if previous_generation != current_generation
                            && let Some(semantic) = &semantic
                        {
                            semantic.schedule();
                        }
                    }
                }
            })
            .map_err(|error| format!("failed to initialize code-index refresh worker: {error}"))?;
        Ok(Self {
            pending,
            wake: Some(wake),
            thread: Some(thread),
        })
    }

    fn schedule(&self, event: FileWatcherEvent) {
        // Cancellation is owned by the semantic job; the lexical worker publishes the next exact
        // generation before asking it to resume.
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .merge(event);
        if let Some(wake) = &self.wake {
            match wake.try_send(()) {
                Ok(()) | Err(std::sync::mpsc::TrySendError::Full(())) => {}
                Err(std::sync::mpsc::TrySendError::Disconnected(())) => {
                    log::warn!("code-index refresh worker stopped unexpectedly");
                }
            }
        }
    }
}

impl Drop for CodeIndexRefreshWorker {
    fn drop(&mut self) {
        self.wake.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Default)]
pub(super) struct FileSystemWatcher {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl FileSystemWatcher {
    pub(super) fn start(
        workspace: WorkspaceRoot,
        updates: Arc<UpdateBroker>,
    ) -> Result<Self, FileSystemWatcherError> {
        Self::start_inner(workspace, updates, FileSystemWatcherObservers::None)
    }

    pub(super) fn start_with_observers(
        workspace: WorkspaceRoot,
        updates: Arc<UpdateBroker>,
        code_index: Arc<CodeIndexRuntime>,
        code_index_semantic: Option<Arc<SemanticIndexJobController>>,
        changes: Arc<dyn WorkspaceFileChangeSink>,
    ) -> Result<Self, FileSystemWatcherError> {
        Self::start_inner(
            workspace,
            updates,
            FileSystemWatcherObservers::WorkspaceRuntime {
                code_index,
                code_index_semantic,
                changes,
            },
        )
    }

    fn start_inner(
        workspace: WorkspaceRoot,
        updates: Arc<UpdateBroker>,
        observers: FileSystemWatcherObservers,
    ) -> Result<Self, FileSystemWatcherError> {
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let (startup, startup_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("zeta-file-system-watcher".into())
            .spawn(move || watch_workspace(workspace, updates, observers, shutdown_rx, startup))
            .map_err(|error| FileSystemWatcherError(error.to_string()))?;
        match startup_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = shutdown.send(());
                let _ = thread.join();
                return Err(FileSystemWatcherError(error));
            }
            Err(error) => {
                let _ = shutdown.send(());
                let _ = thread.join();
                return Err(FileSystemWatcherError(format!(
                    "filesystem watcher did not become ready: {error}"
                )));
            }
        }
        Ok(Self {
            shutdown: Some(shutdown),
            thread: Some(thread),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileSystemWatcherError(String);

impl fmt::Display for FileSystemWatcherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for FileSystemWatcherError {}

impl Drop for FileSystemWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn watch_workspace(
    workspace: WorkspaceRoot,
    updates: Arc<UpdateBroker>,
    observers: FileSystemWatcherObservers,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    startup: std::sync::mpsc::SyncSender<Result<(), String>>,
) {
    let (code_index, code_index_semantic, changes) = match observers {
        FileSystemWatcherObservers::None => (None, None, None),
        FileSystemWatcherObservers::WorkspaceRuntime {
            code_index,
            code_index_semantic,
            changes,
        } => (Some(code_index), code_index_semantic, Some(changes)),
    };
    let Ok(tokio_runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        let _ = startup.send(Err("failed to initialize filesystem watcher runtime".into()));
        return;
    };
    tokio_runtime.block_on(async move {
        let file_watcher = match FileWatcher::new_with_backend(watcher_backend(&workspace)) {
            Ok(file_watcher) => file_watcher,
            Err(error) => {
                let _ = startup.send(Err(format!(
                    "failed to initialize filesystem watcher backend: {error}"
                )));
                return;
            }
        };
        let file_watcher = Arc::new(file_watcher);
        let (subscriber, receiver) = file_watcher.add_subscriber();
        let registration = subscriber.register_paths(vec![
            WatchPath {
                path: workspace.requested_path().to_path_buf(),
                recursive: true,
            },
            WatchPath {
                path: workspace.canonical_path().to_path_buf(),
                recursive: true,
            },
        ]);
        let _registration = match registration {
            Ok(registration) => registration,
            Err(error) => {
                let _ = startup.send(Err(format!(
                    "failed to register filesystem watcher root: {error}"
                )));
                return;
            }
        };
        let code_index_worker = match code_index {
            Some(code_index) => {
                match CodeIndexRefreshWorker::start(code_index, code_index_semantic) {
                    Ok(worker) => Some(worker),
                    Err(error) => {
                        let _ = startup.send(Err(error));
                        return;
                    }
                }
            }
            None => None,
        };
        if let Some(changes) = &changes {
            changes.files_changed(&FsChanged::RescanRequired);
        }
        if let Some(code_index_worker) = &code_index_worker {
            code_index_worker.schedule(FileWatcherEvent::RescanRequired {
                watched_paths: vec![workspace.canonical_path().to_path_buf()],
            });
        }
        if startup.send(Ok(())).is_err() {
            return;
        }
        let mut receiver = DebouncedWatchReceiver::new(receiver, FILE_SYSTEM_WATCH_DEBOUNCE);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                event = receiver.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    if let Some(code_index_worker) = &code_index_worker {
                        code_index_worker.schedule(event.clone());
                    }
                    if let Some(changed) = project_event(&workspace, event) {
                        if let Some(changes) = &changes {
                            changes.files_changed(&changed);
                        }
                        updates.publish_fs_changed(changed);
                    }
                }
            }
        }
    });
}

fn watcher_backend(workspace: &WorkspaceRoot) -> FileWatcherBackend {
    if workspace.requested_path() == workspace.canonical_path() {
        FileWatcherBackend::Recommended
    } else {
        FileWatcherBackend::Polling {
            interval: ALIASED_PATH_POLL_INTERVAL,
        }
    }
}

fn project_event(workspace: &WorkspaceRoot, event: FileWatcherEvent) -> Option<FsChanged> {
    match event {
        FileWatcherEvent::PathsChanged { paths } => {
            let mut paths = paths
                .into_iter()
                .filter_map(|path| workspace.project_observed_path(path))
                .collect::<Vec<_>>();
            paths.sort();
            paths.dedup();
            (!paths.is_empty()).then_some(FsChanged::PathsChanged { paths })
        }
        FileWatcherEvent::RescanRequired { .. } => Some(FsChanged::RescanRequired),
    }
}

#[cfg(test)]
#[path = "fs_watcher_tests.rs"]
mod tests;
