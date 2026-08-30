use super::turn_changes_runtime::{TurnChangesRuntime, publish_records};
use super::update_broker::UpdateBroker;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_file_watcher::{DebouncedWatchReceiver, FileWatcher, WatchPath};
use zeta_protocol::{ThreadId, TurnId};
use zeta_state::SqliteTurnChangeStore;
use zeta_turn_changes::{CaptureState, TurnChangeLedger, TurnChangeStore};

const WATCH_DEBOUNCE: Duration = Duration::from_millis(75);
const WATCH_STARTUP_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) struct ThreadChangeWatcher {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl ThreadChangeWatcher {
    fn start(
        thread_id: ThreadId,
        roots: Vec<PathBuf>,
        ledger: TurnChangeLedger,
        store: Arc<SqliteTurnChangeStore>,
        updates: Arc<UpdateBroker>,
        active_write_lifecycles: Arc<RwLock<BTreeMap<(ThreadId, TurnId), usize>>>,
    ) -> Result<Self, String> {
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let (startup, startup_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("zeta-turn-change-watcher".into())
            .spawn(move || {
                watch_thread(
                    thread_id,
                    roots,
                    ledger,
                    store,
                    updates,
                    active_write_lifecycles,
                    shutdown_rx,
                    startup,
                )
            })
            .map_err(|error| format!("failed to start Turn change watcher: {error}"))?;
        match startup_rx.recv_timeout(WATCH_STARTUP_TIMEOUT) {
            Ok(Ok(())) => Ok(Self {
                shutdown: Some(shutdown),
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = shutdown.send(());
                let _ = thread.join();
                Err(error)
            }
            Err(error) => {
                let _ = shutdown.send(());
                let _ = thread.join();
                Err(format!("Turn change watcher did not become ready: {error}"))
            }
        }
    }
}

impl Drop for ThreadChangeWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl TurnChangesRuntime {
    pub(super) fn start_watcher(
        &self,
        thread_id: ThreadId,
        roots: impl IntoIterator<Item = PathBuf>,
    ) -> Result<(), String> {
        let mut watchers = self
            .watchers
            .write()
            .map_err(|_| "Turn change watcher lock poisoned".to_string())?;
        if watchers.contains_key(&thread_id) {
            return Ok(());
        }
        let roots = roots.into_iter().collect::<Vec<_>>();
        if roots.is_empty() {
            return Err("Turn change watcher requires at least one managed root".into());
        }
        let watcher = ThreadChangeWatcher::start(
            thread_id.clone(),
            roots,
            self.ledger.clone(),
            Arc::clone(&self.store),
            Arc::clone(&self.updates),
            Arc::clone(&self.active_write_lifecycles),
        )?;
        watchers.insert(thread_id, watcher);
        Ok(())
    }

    pub(super) fn stop_watcher(&self, thread_id: &ThreadId) {
        self.watchers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(thread_id);
    }
}

#[allow(clippy::too_many_arguments)]
fn watch_thread(
    thread_id: ThreadId,
    roots: Vec<PathBuf>,
    ledger: TurnChangeLedger,
    store: Arc<SqliteTurnChangeStore>,
    updates: Arc<UpdateBroker>,
    active_write_lifecycles: Arc<RwLock<BTreeMap<(ThreadId, TurnId), usize>>>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    startup: std::sync::mpsc::SyncSender<Result<(), String>>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        let _ = startup.send(Err(
            "failed to initialize Turn change watcher runtime".into()
        ));
        return;
    };
    runtime.block_on(async move {
        let watcher = match FileWatcher::new() {
            Ok(watcher) => Arc::new(watcher),
            Err(error) => {
                let _ = startup.send(Err(format!(
                    "failed to initialize Turn change watcher backend: {error}"
                )));
                return;
            }
        };
        let (subscriber, receiver) = watcher.add_subscriber();
        let paths = roots
            .into_iter()
            .map(|path| WatchPath {
                path,
                recursive: true,
            })
            .collect();
        let _registration = match subscriber.register_paths(paths) {
            Ok(registration) => registration,
            Err(error) => {
                let _ = startup.send(Err(format!(
                    "failed to register managed Thread worktree watcher: {error}"
                )));
                return;
            }
        };
        if startup.send(Ok(())).is_err() {
            return;
        }
        let mut receiver = DebouncedWatchReceiver::new(receiver, WATCH_DEBOUNCE);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                event = receiver.recv() => {
                    if event.is_none() {
                        break;
                    }
                    refresh_thread(
                        &thread_id,
                        &ledger,
                        store.as_ref(),
                        updates.as_ref(),
                        active_write_lifecycles.as_ref(),
                    );
                }
            }
        }
    });
}

fn refresh_thread(
    thread_id: &ThreadId,
    ledger: &TurnChangeLedger,
    store: &SqliteTurnChangeStore,
    updates: &UpdateBroker,
    active_write_lifecycles: &RwLock<BTreeMap<(ThreadId, TurnId), usize>>,
) {
    let records = match store.list_for_thread(thread_id) {
        Ok(records) => records,
        Err(error) => {
            log::warn!("Turn change watcher could not read ledger: {error}");
            return;
        }
    };
    let mut turns = records
        .iter()
        .filter(|record| record.capture_state == CaptureState::Open)
        .map(|record| (record.session_id.clone(), record.turn_id.clone()))
        .collect::<Vec<_>>();
    turns.sort();
    turns.dedup();
    for (session_id, turn_id) in turns {
        let active = active_write_lifecycles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let active_count = active
            .get(&(thread_id.clone(), turn_id.clone()))
            .copied()
            .unwrap_or(0);
        let refreshed =
            match ledger.refresh_turn(session_id.clone(), thread_id.clone(), turn_id.clone()) {
                Ok(records) => records,
                Err(error) => {
                    log::warn!("Turn change watcher refresh failed: {error}");
                    continue;
                }
            };
        publish_records(updates, &refreshed);
        if active_count > 0 {
            continue;
        }
        let unexplained = refreshed.iter().any(|record| {
            !record.attribution_incomplete
                && record
                    .files
                    .iter()
                    .flat_map(|file| [Some(&file.path), file.previous_path.as_ref()])
                    .flatten()
                    .any(|path| !record.write_paths.contains(path))
        });
        if unexplained {
            match ledger.record_ambiguous_write(
                session_id,
                thread_id.clone(),
                turn_id,
                "filesystem write observed outside a known Tool or Hook lifecycle".into(),
            ) {
                Ok(records) => publish_records(updates, &records),
                Err(error) => log::warn!("Turn change watcher attribution failed: {error}"),
            }
        }
    }
}
