use super::{BuiltInSkillSource, SkillCatalogReload, SkillConfigSnapshotProvider, SkillRuntime};
use crate::server::update_broker::UpdateBroker;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_config::{SkillEnablement, SkillsConfig};
use zeta_protocol::{SkillId, SkillName, SkillSourceId};

struct TestConfig {
    skills: Mutex<SkillsConfig>,
}

impl TestConfig {
    fn new() -> Self {
        Self {
            skills: Mutex::new(SkillsConfig::default()),
        }
    }

    fn set_enablement(&self, skill_id: &SkillId, enablement: SkillEnablement) {
        self.skills
            .lock()
            .unwrap()
            .enablement
            .entry(skill_id.source.clone())
            .or_default()
            .insert(skill_id.name.clone(), enablement);
    }
}

impl SkillConfigSnapshotProvider for TestConfig {
    fn snapshot(&self) -> Result<SkillsConfig, String> {
        Ok(self.skills.lock().unwrap().clone())
    }
}

#[test]
fn built_in_catalog_refreshes_and_preserves_monotonic_runtime_generation() {
    let root = test_directory("built-in-refresh");
    write_skill(&root, "skill-creator", "Create skills");
    let config = Arc::new(TestConfig::new());
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        config,
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let initial = runtime.list(SkillCatalogReload::Cached).unwrap();
    assert_eq!(initial.generation, 1);
    assert_eq!(
        initial.entries[0].catalog_entry.id().name.as_str(),
        "skill-creator"
    );

    write_skill(&root, "skill-creator", "Create and improve skills");
    let refreshed = runtime.list(SkillCatalogReload::Refresh).unwrap();
    assert_eq!(refreshed.generation, 2);
    assert_eq!(
        refreshed.entries[0].catalog_entry.metadata().description(),
        "Create and improve skills"
    );
    assert_eq!(
        runtime
            .list(SkillCatalogReload::Refresh)
            .unwrap()
            .generation,
        2
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn missing_auto_detected_built_in_root_is_visible_as_a_diagnostic() {
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Missing,
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let snapshot = runtime.list(SkillCatalogReload::Cached).unwrap();
    assert!(snapshot.entries.is_empty());
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(
        snapshot.diagnostics[0].source,
        "builtin:skill-source:zeta-release"
    );
    assert_eq!(
        snapshot.diagnostics[0].code,
        zeta_skills::SkillDiagnosticCode::SourceUnavailable
    );
}

#[test]
fn enablement_overlay_changes_projection_without_changing_skill_content() {
    let root = test_directory("enablement");
    write_skill(&root, "skill-creator", "Create skills");
    let config = Arc::new(TestConfig::new());
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        config.clone(),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();
    let skill_id = SkillId::new(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        SkillName::new("skill-creator").unwrap(),
    );

    config.set_enablement(&skill_id, SkillEnablement::Disabled);
    let disabled = runtime.list(SkillCatalogReload::Cached).unwrap();

    assert_eq!(disabled.generation, 2);
    assert_eq!(disabled.entries[0].enablement, SkillEnablement::Disabled);
    let _ = fs::remove_dir_all(root);
}

fn test_directory(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-skills-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_skill(root: &Path, name: &str, description: &str) {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\nInstructions.\n"),
    )
    .unwrap();
}
