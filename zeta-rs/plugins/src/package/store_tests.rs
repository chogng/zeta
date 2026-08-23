use super::*;
use std::fs;

fn package(root: &Path, body: &str) {
    fs::create_dir_all(root.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(root.join("skills/review")).unwrap();
    fs::write(root.join("skills/review/SKILL.md"), body).unwrap();
    fs::write(
        root.join(".zeta-plugin/plugin.json"),
        br#"{
          "schemaVersion": 1,
          "id": "acme/review",
          "version": "1.2.0",
          "displayName": "Acme Review",
          "compatibility": {"zeta": ">=0.1.0"},
          "contributions": {"skills": [{"id": "review", "path": "skills/review"}]},
          "permissions": []
        }"#,
    )
    .unwrap();
}

#[test]
fn local_install_promotes_an_exact_immutable_object_and_is_idempotent() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    package(&source, "# Review");
    let local = LocalPluginPackage::load(&source).unwrap();
    let store = PluginPackageStore::open(temporary.path().join("store")).unwrap();

    let first = store.install_local(&local).unwrap();
    let second = store.install_local(&local).unwrap();

    assert_eq!(first, second);
    assert_eq!(store.read(&first).unwrap().package_digest(), &first.digest);
    assert!(
        fs::read_dir(temporary.path().join("store/staging"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn source_mutation_after_validation_fails_without_promoting_an_object() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    package(&source, "# Review");
    let local = LocalPluginPackage::load(&source).unwrap();
    fs::write(source.join("skills/review/SKILL.md"), "# Changed").unwrap();
    let store = PluginPackageStore::open(temporary.path().join("store")).unwrap();

    let error = store.install_local(&local).unwrap_err();

    assert_eq!(error.kind(), PluginErrorKind::PackageConflict);
    assert!(
        fs::read_dir(temporary.path().join("store/objects"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[cfg(any(unix, windows))]
#[test]
fn hard_link_introduced_after_validation_is_rejected_during_installation() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    package(&source, "# Review");
    let local = LocalPluginPackage::load(&source).unwrap();
    fs::hard_link(
        source.join("skills/review/SKILL.md"),
        source.join("skills/review/linked.md"),
    )
    .unwrap();
    let store = PluginPackageStore::open(temporary.path().join("store")).unwrap();

    let error = store.install_local(&local).unwrap_err();

    assert_eq!(error.kind(), PluginErrorKind::PackageUnsafe);
    assert!(error.to_string().contains("hard-linked"));
    assert!(
        fs::read_dir(temporary.path().join("store/objects"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn activated_package_exposes_its_exact_immutable_object_root() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    package(&source, "# Review");
    let local = LocalPluginPackage::load(&source).unwrap();
    let store = PluginPackageStore::open(temporary.path().join("store")).unwrap();
    let installed = store.install_local(&local).unwrap();

    let activated = store.activate(&installed).unwrap();

    assert_eq!(
        activated.package_root(),
        temporary
            .path()
            .join("store/objects")
            .join(installed.digest.as_str().trim_start_matches("sha256:"))
            .canonicalize()
            .unwrap()
    );
}
