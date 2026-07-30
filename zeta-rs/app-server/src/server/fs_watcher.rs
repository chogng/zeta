use super::update_broker::UpdateBroker;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_file_watcher::{DebouncedWatchReceiver, FileWatcher, FileWatcherEvent, WatchPath};

const FILE_SYSTEM_WATCH_DEBOUNCE: Duration = Duration::from_millis(75);

#[derive(Default)]
pub(super) struct FileSystemWatcher {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl FileSystemWatcher {
    pub(super) fn start(workspace_root: PathBuf, updates: Arc<UpdateBroker>) -> Self {
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-file-system-watcher".into())
            .spawn(move || watch_workspace(workspace_root, updates, shutdown_rx))
            .ok();
        if thread.is_none() {
            return Self::default();
        }
        Self {
            shutdown: Some(shutdown),
            thread,
        }
    }
}

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
    workspace_root: PathBuf,
    updates: Arc<UpdateBroker>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) {
    let Ok(tokio_runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        return;
    };
    tokio_runtime.block_on(async move {
        let Ok(file_watcher) = FileWatcher::new() else {
            return;
        };
        let file_watcher = Arc::new(file_watcher);
        let (subscriber, receiver) = file_watcher.add_subscriber();
        let _registration = subscriber.register_paths(vec![WatchPath {
            path: workspace_root.clone(),
            recursive: true,
        }]);
        let mut receiver = DebouncedWatchReceiver::new(receiver, FILE_SYSTEM_WATCH_DEBOUNCE);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                event = receiver.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    if let Some(changed) = project_event(&workspace_root, event) {
                        updates.publish_fs_changed(changed);
                    }
                }
            }
        }
    });
}

fn project_event(workspace_root: &Path, event: FileWatcherEvent) -> Option<FsChanged> {
    match event {
        FileWatcherEvent::PathsChanged { paths } => {
            let mut paths = paths
                .into_iter()
                .filter_map(|path| {
                    path.strip_prefix(workspace_root)
                        .ok()
                        .map(Path::to_path_buf)
                })
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
