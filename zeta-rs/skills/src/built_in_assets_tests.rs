use crate::{SkillCatalog, SkillSourceId, SkillSourceKind, SkillSourceRoot};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const SKILL_CREATOR: &str = include_str!("../assets/skill-creator/SKILL.md");

#[test]
fn repository_built_in_skills_form_a_valid_catalog() {
    let directory = TestDirectory::new();
    write_skill(directory.path(), "skill-creator", SKILL_CREATOR);
    let source = SkillSourceRoot::built_in(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        directory.path(),
    )
    .unwrap();

    let catalog = SkillCatalog::discover(vec![source]).unwrap();
    let snapshot = catalog.snapshot();

    assert!(snapshot.diagnostics().is_empty());
    assert_eq!(
        snapshot
            .list()
            .iter()
            .map(|entry| entry.id().name.as_str())
            .collect::<Vec<_>>(),
        ["skill-creator"]
    );
    assert!(
        snapshot
            .list()
            .iter()
            .all(|entry| entry.source().kind() == SkillSourceKind::BuiltIn)
    );
    assert!(
        snapshot
            .list()
            .iter()
            .all(|entry| entry.source().allows_automatic_activation())
    );
}

fn write_skill(root: &Path, name: &str, contents: &str) {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("SKILL.md"), contents).unwrap();
}

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-built-in-skills-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
