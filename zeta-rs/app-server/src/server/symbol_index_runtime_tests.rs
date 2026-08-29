use std::fs;
use std::sync::Arc;

use tempfile::TempDir;
use zeta_codebase::Codebase;
use zeta_codebase::CodebaseLimits;
use zeta_codebase::CodebaseStorage;
use zeta_codebase::SymbolIndexQuery;
use zeta_codebase::SymbolIndexStorage;
use zeta_workspace::WorkspaceRoot;

use super::SymbolIndexRuntime;
use super::SymbolIndexRuntimeState;

fn workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("workspace");
    fs::create_dir(directory.path().join(".git")).expect("git marker");
    directory
}

fn codebase(directory: &TempDir) -> Arc<Codebase> {
    let index = Codebase::open(
        WorkspaceRoot::open(directory.path()).expect("workspace root"),
        CodebaseStorage::Memory,
        CodebaseLimits::default(),
    )
    .expect("Codebase");
    index.rebuild().expect("codebase rebuild");
    Arc::new(index)
}

#[test]
fn reconcile_publishes_a_searchable_generation() {
    let directory = workspace();
    fs::write(directory.path().join("lib.rs"), "pub fn searchable() {}\n").expect("source");
    let runtime = SymbolIndexRuntime::open(codebase(&directory), SymbolIndexStorage::Memory)
        .expect("symbol-index runtime");

    assert_eq!(runtime.state(), SymbolIndexRuntimeState::Empty);
    let snapshot = runtime.reconcile().expect("reconcile");
    assert_eq!(snapshot.indexed_symbol_count, 1);
    assert!(matches!(runtime.state(), SymbolIndexRuntimeState::Ready(_)));
    assert_eq!(
        runtime
            .search(&SymbolIndexQuery::new("searchable"))
            .unwrap()[0]
            .symbol
            .name,
        "searchable"
    );
}

#[test]
fn search_marks_a_projection_stale_after_source_generation_changes() {
    let directory = workspace();
    let source = directory.path().join("lib.rs");
    fs::write(&source, "pub fn before() {}\n").expect("source");
    let codebase = codebase(&directory);
    let runtime = SymbolIndexRuntime::open(Arc::clone(&codebase), SymbolIndexStorage::Memory)
        .expect("symbol-index runtime");
    runtime.reconcile().expect("initial reconcile");

    fs::write(&source, "pub fn after() {}\n").expect("changed source");
    codebase
        .refresh_observed_paths(&[source])
        .expect("codebase refresh");
    let _ = runtime.search(&SymbolIndexQuery::new("before"));

    assert!(matches!(runtime.state(), SymbolIndexRuntimeState::Stale(_)));
}
