use super::update_broker::UpdateBroker;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_config::{ConfigChange, SkillEnablement, SkillSourceEnablement, SkillsConfig};
use zeta_file_watcher::{DebouncedWatchReceiver, FileWatcher, WatchPath};
use zeta_skills::{
    SkillCatalog, SkillCatalogEntry, SkillDiagnosticCode, SkillSourceId, SkillSourceRoot,
};

const BUILT_IN_SOURCE_ID: &str = "builtin:skill-source:zeta-release";

/// Supplies the latest resolved user Skill configuration to the App Server-owned runtime.
///
/// Implementations resolve configuration authority before returning and must not convert an
/// untrusted client path directly into a validated [`SkillSourceRoot`].
pub(crate) trait SkillConfigSnapshotProvider: Send + Sync {
    fn snapshot(&self) -> Result<SkillsConfig, String>;

    fn config_changes(&self) -> Option<std::sync::mpsc::Receiver<ConfigChange>> {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkillCatalogReload {
    Cached,
    Refresh,
}

pub(crate) enum BuiltInSkillSource {
    Root(PathBuf),
    Missing,
    Omitted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SkillRuntimeEntry {
    pub(crate) catalog_entry: SkillCatalogEntry,
    pub(crate) enablement: SkillEnablement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SkillRuntimeDiagnostic {
    pub(crate) source: String,
    pub(crate) subject: Option<String>,
    pub(crate) code: SkillDiagnosticCode,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SkillRuntimeSnapshot {
    pub(crate) generation: u64,
    pub(crate) entries: Vec<SkillRuntimeEntry>,
    pub(crate) diagnostics: Vec<SkillRuntimeDiagnostic>,
}

pub(crate) struct SkillRuntime {
    built_in_source: BuiltInSkillSource,
    config: Arc<dyn SkillConfigSnapshotProvider>,
    state: Mutex<SkillRuntimeState>,
    updates: Arc<UpdateBroker>,
}

#[derive(Default)]
pub(super) struct SkillWatcher {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

struct SkillRuntimeState {
    source_fingerprint: Vec<SourceFingerprint>,
    catalog: SkillCatalog,
    snapshot: Arc<SkillRuntimeSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceFingerprint {
    id: SkillSourceId,
    root: PathBuf,
}

struct SourceComposition {
    fingerprint: Vec<SourceFingerprint>,
    roots: Vec<SkillSourceRoot>,
    diagnostics: Vec<SkillRuntimeDiagnostic>,
}

#[derive(Clone, Copy)]
enum SourceKind {
    BuiltIn,
    User,
}

impl SkillRuntime {
    pub(super) fn new(
        built_in_source: BuiltInSkillSource,
        config: Arc<dyn SkillConfigSnapshotProvider>,
        updates: Arc<UpdateBroker>,
    ) -> Result<Arc<Self>, String> {
        let skills_config = config.snapshot()?;
        let composition = compose_sources(&built_in_source, &skills_config)?;
        let catalog = SkillCatalog::discover(composition.roots)
            .map_err(|error| format!("failed to discover Skill catalog: {error}"))?;
        let snapshot = Arc::new(project_snapshot(
            1,
            catalog.snapshot().as_ref(),
            &skills_config,
            composition.diagnostics,
        ));
        Ok(Arc::new(Self {
            built_in_source,
            config,
            state: Mutex::new(SkillRuntimeState {
                source_fingerprint: composition.fingerprint,
                catalog,
                snapshot,
            }),
            updates,
        }))
    }

    pub(crate) fn list(
        &self,
        reload: SkillCatalogReload,
    ) -> Result<Arc<SkillRuntimeSnapshot>, String> {
        self.reconcile(reload)
    }

    pub(crate) fn watched_paths(&self) -> Vec<PathBuf> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut paths = state
            .source_fingerprint
            .iter()
            .map(|source| source.root.clone())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    pub(super) fn start_watching(self: &Arc<Self>) -> SkillWatcher {
        let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel();
        let (ready, ready_rx) = std::sync::mpsc::channel();
        let runtime = Arc::downgrade(self);
        let thread = std::thread::Builder::new()
            .name("zeta-skill-watcher".into())
            .spawn(move || watch_skill_sources(runtime, shutdown_rx, ready))
            .ok();
        if thread.is_none() {
            return SkillWatcher::default();
        }
        let _ = ready_rx.recv_timeout(Duration::from_secs(1));
        SkillWatcher {
            shutdown: Some(shutdown),
            thread,
        }
    }

    fn reconcile(&self, reload: SkillCatalogReload) -> Result<Arc<SkillRuntimeSnapshot>, String> {
        let skills_config = self.config.snapshot()?;
        let composition = compose_sources(&self.built_in_source, &skills_config)?;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let catalog_snapshot = if state.source_fingerprint != composition.fingerprint {
            state.catalog = SkillCatalog::discover(composition.roots)
                .map_err(|error| format!("failed to rebuild Skill catalog: {error}"))?;
            state.source_fingerprint = composition.fingerprint;
            state.catalog.snapshot()
        } else {
            match reload {
                SkillCatalogReload::Cached => state.catalog.snapshot(),
                SkillCatalogReload::Refresh => state.catalog.refresh(),
            }
        };
        let next = project_snapshot(
            state
                .snapshot
                .generation
                .checked_add(1)
                .expect("Skill runtime generation overflowed"),
            catalog_snapshot.as_ref(),
            &skills_config,
            composition.diagnostics,
        );
        if same_projection(state.snapshot.as_ref(), &next) {
            return Ok(Arc::clone(&state.snapshot));
        }
        state.snapshot = Arc::new(next);
        let snapshot = Arc::clone(&state.snapshot);
        drop(state);
        self.updates.publish_skills_changed(snapshot.generation);
        Ok(snapshot)
    }
}

impl Drop for SkillWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn watch_skill_sources(
    runtime: std::sync::Weak<SkillRuntime>,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ready: std::sync::mpsc::Sender<()>,
) {
    let Ok(tokio_runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    else {
        return;
    };
    let config_changes = runtime
        .upgrade()
        .and_then(|runtime| runtime.config.config_changes());
    tokio_runtime.block_on(async move {
        let Ok(file_watcher) = FileWatcher::new() else {
            return;
        };
        let file_watcher = Arc::new(file_watcher);
        let (subscriber, receiver) = file_watcher.add_subscriber();
        let Some(skill_runtime) = runtime.upgrade() else {
            return;
        };
        let mut watched_paths = skill_runtime.watched_paths();
        drop(skill_runtime);
        let mut registration = subscriber.register_paths(watch_paths(&watched_paths));
        let mut receiver = DebouncedWatchReceiver::new(receiver, Duration::from_millis(75));
        let mut config_poll = tokio::time::interval(Duration::from_millis(250));
        let _ = ready.send(());
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                _ = config_poll.tick() => {
                    let Some(changes) = &config_changes else {
                        continue;
                    };
                    if changes.try_iter().count() == 0 {
                        continue;
                    }
                    let Some(skill_runtime) = runtime.upgrade() else {
                        break;
                    };
                    let _ = skill_runtime.list(SkillCatalogReload::Cached);
                    let next_paths = skill_runtime.watched_paths();
                    if next_paths != watched_paths {
                        drop(registration);
                        registration = subscriber.register_paths(watch_paths(&next_paths));
                        watched_paths = next_paths;
                    }
                }
                event = receiver.recv() => {
                    if event.is_none() {
                        break;
                    }
                    let Some(skill_runtime) = runtime.upgrade() else {
                        break;
                    };
                    let _ = skill_runtime.list(SkillCatalogReload::Refresh);
                    let next_paths = skill_runtime.watched_paths();
                    if next_paths != watched_paths {
                        drop(registration);
                        registration = subscriber.register_paths(watch_paths(&next_paths));
                        watched_paths = next_paths;
                    }
                }
            }
        }
        drop(registration);
    });
}

fn watch_paths(paths: &[PathBuf]) -> Vec<WatchPath> {
    paths
        .iter()
        .map(|path| WatchPath {
            path: path.clone(),
            recursive: path.is_dir(),
        })
        .collect()
}

fn compose_sources(
    built_in_source: &BuiltInSkillSource,
    config: &SkillsConfig,
) -> Result<SourceComposition, String> {
    let mut fingerprint = Vec::new();
    let mut roots = Vec::new();
    let mut diagnostics = Vec::new();
    let built_in_id = SkillSourceId::new(BUILT_IN_SOURCE_ID)
        .expect("the repository built-in Skill source ID is valid");
    match built_in_source {
        BuiltInSkillSource::Root(root) => add_source(
            built_in_id,
            root.clone(),
            SourceKind::BuiltIn,
            &mut fingerprint,
            &mut roots,
            &mut diagnostics,
        ),
        BuiltInSkillSource::Missing => diagnostics.push(source_unavailable_diagnostic(
            &built_in_id,
            "the bundled Skill directory could not be located",
        )),
        BuiltInSkillSource::Omitted => {}
    }
    for source in config
        .sources
        .values()
        .filter(|source| source.enablement == SkillSourceEnablement::Enabled)
    {
        let root = PathBuf::from(&source.root_reference);
        if !root.is_absolute() {
            diagnostics.push(source_unavailable_diagnostic(
                &source.id,
                "local Skill source root reference must be an absolute path",
            ));
            continue;
        }
        add_source(
            source.id.clone(),
            root,
            SourceKind::User,
            &mut fingerprint,
            &mut roots,
            &mut diagnostics,
        );
    }
    fingerprint.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(SourceComposition {
        fingerprint,
        roots,
        diagnostics,
    })
}

fn add_source(
    id: SkillSourceId,
    root: PathBuf,
    kind: SourceKind,
    fingerprint: &mut Vec<SourceFingerprint>,
    roots: &mut Vec<SkillSourceRoot>,
    diagnostics: &mut Vec<SkillRuntimeDiagnostic>,
) {
    let source = match kind {
        SourceKind::BuiltIn => SkillSourceRoot::built_in(id.clone(), &root),
        SourceKind::User => SkillSourceRoot::user(id.clone(), &root),
    };
    match source {
        Ok(source) => {
            fingerprint.push(SourceFingerprint { id, root });
            roots.push(source);
        }
        Err(error) => diagnostics.push(source_unavailable_diagnostic(&id, &error.to_string())),
    }
}

fn project_snapshot(
    generation: u64,
    catalog: &zeta_skills::SkillCatalogSnapshot,
    config: &SkillsConfig,
    mut runtime_diagnostics: Vec<SkillRuntimeDiagnostic>,
) -> SkillRuntimeSnapshot {
    let entries = catalog
        .list()
        .iter()
        .cloned()
        .map(|catalog_entry| SkillRuntimeEntry {
            enablement: config.skill_enablement(catalog_entry.id()),
            catalog_entry,
        })
        .collect();
    runtime_diagnostics.extend(catalog.diagnostics().iter().map(|diagnostic| {
        SkillRuntimeDiagnostic {
            source: diagnostic.source().to_string(),
            subject: diagnostic.subject().map(str::to_owned),
            code: diagnostic.code(),
            message: diagnostic.message().to_owned(),
        }
    }));
    runtime_diagnostics.sort_by(|left, right| {
        (&left.source, &left.subject, left.code, &left.message).cmp(&(
            &right.source,
            &right.subject,
            right.code,
            &right.message,
        ))
    });
    runtime_diagnostics.dedup();
    SkillRuntimeSnapshot {
        generation,
        entries,
        diagnostics: runtime_diagnostics,
    }
}

fn same_projection(current: &SkillRuntimeSnapshot, next: &SkillRuntimeSnapshot) -> bool {
    current.entries == next.entries && current.diagnostics == next.diagnostics
}

fn source_unavailable_diagnostic(source: &SkillSourceId, message: &str) -> SkillRuntimeDiagnostic {
    SkillRuntimeDiagnostic {
        source: source.to_string(),
        subject: None,
        code: SkillDiagnosticCode::SourceUnavailable,
        message: message.to_owned(),
    }
}

#[cfg(test)]
#[path = "skills_runtime_tests.rs"]
mod tests;
