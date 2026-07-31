use super::update_broker::UpdateBroker;
use std::fmt;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_file_watcher::{
    DebouncedWatchReceiver, FileWatcher, FileWatcherBackend, FileWatcherEvent, WatchPath,
};
use zeta_workspace::WorkspaceRoot;

const FILE_SYSTEM_WATCH_DEBOUNCE: Duration = Duration::from_millis(75);
const ALIASED_PATH_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let (startup, startup_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("zeta-file-system-watcher".into())
            .spawn(move || watch_workspace(workspace, updates, shutdown_rx, startup))
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
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    startup: std::sync::mpsc::SyncSender<Result<(), String>>,
) {
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
                    if let Some(changed) = project_event(&workspace, event) {
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
