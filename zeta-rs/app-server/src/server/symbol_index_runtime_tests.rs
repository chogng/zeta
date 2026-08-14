use std::fs;
use std::sync::Arc;

use tempfile::TempDir;
use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexLimits;
use zeta_code_index::CodeIndexStorage;
use zeta_symbol_index::SymbolIndexQuery;
use zeta_symbol_index::SymbolIndexStorage;
use zeta_workspace::WorkspaceRoot;

use super::SymbolIndexRuntime;
use super::SymbolIndexRuntimeState;

fn workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("workspace");
    fs::create_dir(directory.path().join(".git")).expect("git marker");
    directory
}

fn code_index(directory: &TempDir) -> Arc<CodeIndex> {
    let index = CodeIndex::open(
        WorkspaceRoot::open(directory.path()).expect("workspace root"),
        CodeIndexStorage::Memory,
        CodeIndexLimits::default(),
    )
    .expect("code index");
    index.rebuild().expect("code-index rebuild");
    Arc::new(index)
}

#[test]
fn reconcile_publishes_a_searchable_generation() {
    let directory = workspace();
    fs::write(directory.path().join("lib.rs"), "pub fn searchable() {}\n").expect("source");
    let runtime = SymbolIndexRuntime::open(code_index(&directory), SymbolIndexStorage::Memory)
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
    let code_index = code_index(&directory);
    let runtime = SymbolIndexRuntime::open(Arc::clone(&code_index), SymbolIndexStorage::Memory)
        .expect("symbol-index runtime");
    runtime.reconcile().expect("initial reconcile");

    fs::write(&source, "pub fn after() {}\n").expect("changed source");
    code_index
        .refresh_observed_paths(&[source])
        .expect("code-index refresh");
    let _ = runtime.search(&SymbolIndexQuery::new("before"));

    assert!(matches!(runtime.state(), SymbolIndexRuntimeState::Stale(_)));
}
