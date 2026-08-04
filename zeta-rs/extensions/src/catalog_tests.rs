use super::*;
use std::fs;

#[test]
fn discovers_manifest_and_reads_a_grammar_resource() {
    let root = tempfile::tempdir().expect("extension root");
    let package = root.path().join("zeta.demo");
    fs::create_dir_all(package.join("syntaxes")).expect("package directories");
    fs::write(
        package.join("package.json"),
        r#"{
            "name": "demo",
            "publisher": "zeta",
            "version": "1.0.0",
            "displayName": "Demo",
            "contributes": {
                "grammars": [{
                    "language": "demo",
                    "scopeName": "source.demo",
                    "path": "./syntaxes/demo.tmLanguage.json"
                }]
            }
        }"#,
    )
    .expect("manifest");
    fs::write(
        package.join("syntaxes/demo.tmLanguage.json"),
        br#"{"scopeName":"source.demo","patterns":[]}"#,
    )
    .expect("grammar");

    let mut catalog = ExtensionCatalog::new(vec![ExtensionRoot::user(root.path())]);
    let snapshot = catalog.list(ExtensionCatalogReload::Refresh);

    assert_eq!(snapshot.extensions.len(), 1);
    assert_eq!(snapshot.extensions[0].id, "zeta.demo");
    assert!(snapshot.diagnostics.is_empty());
    let resource = catalog
        .open_resource("zeta.demo", "syntaxes/demo.tmLanguage.json")
        .expect("grammar resource");
    assert_eq!(resource.mime_type, "application/json");
    assert_eq!(
        resource.bytes,
        br#"{"scopeName":"source.demo","patterns":[]}"#
    );
}

#[test]
fn reports_invalid_manifests_without_registering_them() {
    let root = tempfile::tempdir().expect("extension root");
    let package = root.path().join("broken");
    fs::create_dir_all(&package).expect("package directory");
    fs::write(package.join("package.json"), r#"{"name":"demo"}"#).expect("manifest");

    let mut catalog = ExtensionCatalog::new(vec![ExtensionRoot::user(root.path())]);
    let snapshot = catalog.list(ExtensionCatalogReload::Refresh);

    assert!(snapshot.extensions.is_empty());
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(
        snapshot.diagnostics[0].code,
        ExtensionDiagnosticCode::InvalidManifest
    );
}

#[test]
fn rejects_paths_that_escape_the_package() {
    let root = tempfile::tempdir().expect("extension root");
    let package = root.path().join("zeta.demo");
    fs::create_dir_all(&package).expect("package directory");
    fs::write(
        package.join("package.json"),
        r#"{"name":"demo","publisher":"zeta","version":"1.0.0"}"#,
    )
    .expect("manifest");

    let mut catalog = ExtensionCatalog::new(vec![ExtensionRoot::user(root.path())]);
    catalog.list(ExtensionCatalogReload::Refresh);

    assert_eq!(
        catalog.open_resource("zeta.demo", "../package.json"),
        Err(ExtensionCatalogError::InvalidPath)
    );
}
