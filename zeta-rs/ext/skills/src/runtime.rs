use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_config::ConfigChange;
use zeta_config::SkillEnablement;
use zeta_config::SkillSourceEnablement;
use zeta_config::SkillsConfig;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillRef;
use zeta_skills::ActivatedSkill;
use zeta_skills::SkillCatalog;
use zeta_skills::SkillCatalogEntry;
use zeta_skills::SkillCompatibility;
use zeta_skills::SkillDiagnosticCode;
use zeta_skills::SkillResource;
use zeta_skills::SkillResourcePath;
use zeta_skills::SkillSourceId;
use zeta_skills::SkillSourceRoot;

const BUILT_IN_SOURCE_ID: &str = "builtin:skill-source:zeta-release";
const WORKSPACE_SOURCE_ID: &str = "workspace:skill-source:.zeta";

/// Supplies resolved Skill configuration to the shared Skill runtime.
///
/// Implementations must resolve configuration authority before returning and must not convert an
/// untrusted client path directly into a validated [`SkillSourceRoot`].
pub trait SkillConfigSnapshotProvider: Send + Sync {
    fn snapshot(&self) -> Result<SkillsConfig, String>;

    fn config_changes(&self) -> Option<std::sync::mpsc::Receiver<ConfigChange>> {
        None
    }
}

/// Receives generation changes without coupling the Skill runtime to a transport or product host.
pub trait SkillRuntimeEventSink: Send + Sync {
    fn skills_changed(&self, generation: u64);
}

#[cfg(test)]
pub(crate) struct NoSkillRuntimeEvents;

#[cfg(test)]
impl SkillRuntimeEventSink for NoSkillRuntimeEvents {
    fn skills_changed(&self, _: u64) {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillCatalogReload {
    Cached,
    Refresh,
}

pub enum BuiltInSkillSource {
    Root(PathBuf),
    Missing,
    Omitted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRuntimeEntry {
    pub catalog_entry: SkillCatalogEntry,
    pub enablement: SkillEnablement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRuntimeDiagnostic {
    pub source: String,
    pub subject: Option<String>,
    pub code: SkillDiagnosticCode,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillRuntimeSnapshot {
    pub generation: u64,
    pub entries: Vec<SkillRuntimeEntry>,
    pub diagnostics: Vec<SkillRuntimeDiagnostic>,
}

pub struct SkillRuntime {
    built_in_source: BuiltInSkillSource,
    pub(crate) config: Arc<dyn SkillConfigSnapshotProvider>,
    pub(crate) workspace_root: Mutex<Option<PathBuf>>,
    pub(crate) state: Mutex<SkillRuntimeState>,
    events: Arc<dyn SkillRuntimeEventSink>,
}

pub(crate) struct SkillRuntimeState {
    pub(crate) source_fingerprint: Vec<SourceFingerprint>,
    catalog: SkillCatalog,
    snapshot: Arc<SkillRuntimeSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceFingerprint {
    id: SkillSourceId,
    pub(crate) root: PathBuf,
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
    Workspace,
}

impl SkillRuntime {
    pub fn new(
        built_in_source: BuiltInSkillSource,
        config: Arc<dyn SkillConfigSnapshotProvider>,
        events: Arc<dyn SkillRuntimeEventSink>,
    ) -> Result<Arc<Self>, String> {
        let skills_config = config.snapshot()?;
        let composition = compose_sources(&built_in_source, &skills_config, None)?;
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
            workspace_root: Mutex::new(None),
            state: Mutex::new(SkillRuntimeState {
                source_fingerprint: composition.fingerprint,
                catalog,
                snapshot,
            }),
            events,
        }))
    }

    pub fn list(&self, reload: SkillCatalogReload) -> Result<Arc<SkillRuntimeSnapshot>, String> {
        self.reconcile(reload)
    }

    pub fn activate_explicit(&self, selected: &SkillRef) -> Result<ActivatedSkill, String> {
        self.activate_available(selected, SkillActivationReason::Explicit)
    }

    pub(crate) fn activate_model_selected(
        &self,
        selected: &SkillRef,
    ) -> Result<ActivatedSkill, String> {
        self.activate_available(selected, SkillActivationReason::Automatic)
    }

    /// Reads one inert package resource from an enabled, compatible Skill pinned to an exact
    /// `SKILL.md` digest.
    ///
    /// Hosts may materialize the returned bytes into their ordinary resource store. This method
    /// never infers a media type, executes scripts, or grants write/publish authority.
    pub fn read_resource(
        &self,
        selected: &SkillRef,
        path: &SkillResourcePath,
    ) -> Result<SkillResource, String> {
        self.reconcile(SkillCatalogReload::Refresh)?;
        let state = self
            .state
            .lock()
            .map_err(|_| "Skill runtime lock poisoned".to_string())?;
        let entry = state
            .snapshot
            .entries
            .iter()
            .find(|entry| entry.catalog_entry.id() == &selected.id)
            .ok_or_else(|| format!("Skill '{}' is not available", selected.id.name))?;
        if entry.enablement != SkillEnablement::Enabled {
            return Err(format!("Skill '{}' is disabled", selected.id.name));
        }
        if !matches!(
            entry.catalog_entry.compatibility(),
            SkillCompatibility::Compatible
        ) {
            return Err(format!("Skill '{}' is not compatible", selected.id.name));
        }
        state
            .catalog
            .read_resource(selected, path)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn read_model_resource(
        &self,
        selected: &SkillRef,
        path: &SkillResourcePath,
    ) -> Result<SkillResource, String> {
        self.read_resource(selected, path)
    }

    fn activate_available(
        &self,
        selected: &SkillRef,
        reason: SkillActivationReason,
    ) -> Result<ActivatedSkill, String> {
        self.reconcile(SkillCatalogReload::Refresh)?;
        let state = self
            .state
            .lock()
            .map_err(|_| "Skill runtime lock poisoned".to_string())?;
        let entry = state
            .snapshot
            .entries
            .iter()
            .find(|entry| entry.catalog_entry.id() == &selected.id)
            .ok_or_else(|| format!("Skill '{}' is not available", selected.id.name))?;
        if entry.enablement != SkillEnablement::Enabled {
            return Err(format!("Skill '{}' is disabled", selected.id.name));
        }
        if !matches!(
            entry.catalog_entry.compatibility(),
            SkillCompatibility::Compatible
        ) {
            return Err(format!("Skill '{}' is not compatible", selected.id.name));
        }
        state
            .catalog
            .activate(selected, reason)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn load_frozen(
        &self,
        frozen: &FrozenSkillActivation,
    ) -> Result<ActivatedSkill, String> {
        self.reconcile(SkillCatalogReload::Refresh)?;
        let state = self
            .state
            .lock()
            .map_err(|_| "Skill runtime lock poisoned".to_string())?;
        state
            .catalog
            .activate(
                &SkillRef::pinned(frozen.id.clone(), frozen.content_digest.clone()),
                frozen.reason,
            )
            .map_err(|error| error.to_string())
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
        drop(state);
        if let Some(workspace_root) = self
            .workspace_root
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            let metadata_root = workspace_root.join(".zeta");
            paths.push(if metadata_root.is_dir() {
                metadata_root
            } else {
                workspace_root.clone()
            });
        }
        paths.sort();
        paths.dedup();
        paths
    }

    /// Rebinds the native `.zeta/skills` source for the active Workspace.
    pub fn bind_workspace_root(
        &self,
        workspace_root: PathBuf,
    ) -> Result<Arc<SkillRuntimeSnapshot>, String> {
        let previous = {
            let mut current = self
                .workspace_root
                .lock()
                .map_err(|_| "Workspace Skill source lock poisoned".to_string())?;
            current.replace(workspace_root)
        };
        match self.reconcile(SkillCatalogReload::Refresh) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                *self
                    .workspace_root
                    .lock()
                    .map_err(|_| "Workspace Skill source lock poisoned".to_string())? = previous;
                Err(error)
            }
        }
    }

    fn reconcile(&self, reload: SkillCatalogReload) -> Result<Arc<SkillRuntimeSnapshot>, String> {
        let skills_config = self.config.snapshot()?;
        let workspace_root = self
            .workspace_root
            .lock()
            .map_err(|_| "Workspace Skill source lock poisoned".to_string())?
            .clone();
        let composition = compose_sources(
            &self.built_in_source,
            &skills_config,
            workspace_root.as_deref(),
        )?;
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
        self.events.skills_changed(snapshot.generation);
        Ok(snapshot)
    }
}

fn compose_sources(
    built_in_source: &BuiltInSkillSource,
    config: &SkillsConfig,
    workspace_root: Option<&Path>,
) -> Result<SourceComposition, String> {
    let mut fingerprint = Vec::new();
    let mut roots = Vec::new();
    let mut diagnostics = Vec::new();
    let built_in_id = SkillSourceId::new(BUILT_IN_SOURCE_ID)
        .expect("the repository built-in Skill source ID is valid");
    let workspace_id = SkillSourceId::new(WORKSPACE_SOURCE_ID)
        .expect("the native Workspace Skill source ID is valid");
    match built_in_source {
        BuiltInSkillSource::Root(root) => add_source(
            built_in_id.clone(),
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
        if source.id == built_in_id || source.id == workspace_id {
            diagnostics.push(source_unavailable_diagnostic(
                &source.id,
                "native Skill source identities cannot be registered as user sources",
            ));
            continue;
        }
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
    if let Some(workspace_root) = workspace_root {
        let root = workspace_root.join(".zeta/skills");
        match std::fs::symlink_metadata(&root) {
            Ok(_) => add_source(
                workspace_id.clone(),
                root,
                SourceKind::Workspace,
                &mut fingerprint,
                &mut roots,
                &mut diagnostics,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => diagnostics.push(source_unavailable_diagnostic(
                &workspace_id,
                "the native Workspace Skill source cannot be inspected",
            )),
        }
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
        SourceKind::Workspace => SkillSourceRoot::workspace(id.clone(), &root),
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
