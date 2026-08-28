use super::*;
use std::fs;
use tempfile::TempDir;

fn workspace() -> TempDir {
    let directory = tempfile::tempdir().expect("workspace");
    fs::create_dir(directory.path().join(".git")).expect("git marker");
    directory
}

fn search(directory: &TempDir, storage: FastRegexSearchStorage) -> FastRegexSearch {
    FastRegexSearch::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage,
        FastRegexSearchLimits::default(),
    )
    .expect("fast regex search")
}

fn query(pattern: &str) -> FastRegexQuery {
    FastRegexQuery {
        query: pattern.into(),
        pattern: FastRegexPattern::Regex,
        case_sensitivity: FastRegexCaseSensitivity::Sensitive,
        include_patterns: Vec::new(),
        exclude_patterns: Vec::new(),
        max_results: 20,
    }
}

#[test]
fn sparse_candidates_are_verified_by_the_regex_engine() {
    let directory = workspace();
    fs::write(
        directory.path().join("auth.rs"),
        "let authentication_value = token;\nlet authentication_value = other;\n",
    )
    .expect("source");
    fs::write(
        directory.path().join("noise.rs"),
        "authentication only\ntoken only\n",
    )
    .expect("noise");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let result = index
        .search(&query(r"authentication_.*token"))
        .expect("search");

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].path, Path::new("auth.rs"));
    assert_eq!(result.matches[0].line_number, 1);
    assert_eq!(
        result.matches[0].ranges,
        [FastRegexRange {
            start_byte: 4,
            end_byte: 32
        }]
    );
}

#[test]
fn alternation_and_short_patterns_never_drop_valid_files() {
    let directory = workspace();
    fs::write(directory.path().join("first.txt"), "pikachu\n").expect("first");
    fs::write(directory.path().join("second.txt"), "raichu\n").expect("second");
    fs::write(directory.path().join("short.txt"), "a\n").expect("short");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let alternatives = index
        .search(&query(r"(pika|rai)chu"))
        .expect("alternatives");
    let short = index.search(&query("a")).expect("short");

    assert_eq!(
        alternatives
            .matches
            .iter()
            .map(|item| &item.path)
            .collect::<Vec<_>>(),
        [Path::new("first.txt"), Path::new("second.txt")]
    );
    assert_eq!(short.matches.len(), 3);
}

#[test]
fn refresh_and_overlay_keep_exact_search_current() {
    let directory = workspace();
    let path = directory.path().join("current.rs");
    fs::write(&path, "before_marker\n").expect("before");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    fs::write(&path, "after_marker\n").expect("after");
    index.refresh_observed_paths(&[path]).expect("refresh");
    index
        .synchronize_overlay(PathBuf::from("current.rs"), "unsaved_marker\n".into())
        .expect("overlay");

    assert_eq!(
        index.search(&query("after_marker")).unwrap().matches.len(),
        0
    );
    assert_eq!(
        index
            .search(&query("unsaved_marker"))
            .unwrap()
            .matches
            .len(),
        1
    );
    index.close_overlay(Path::new("current.rs")).expect("close");
    assert_eq!(
        index.search(&query("after_marker")).unwrap().matches.len(),
        1
    );
}

#[test]
fn persistent_storage_writes_compact_index_files() {
    let directory = workspace();
    fs::write(directory.path().join("source.rs"), "persistent_marker\n").expect("source");
    let storage = tempfile::tempdir().expect("storage");
    let index = search(
        &directory,
        FastRegexSearchStorage::Persistent(storage.path().to_path_buf()),
    );

    index.rebuild().expect("rebuild");

    assert!(storage.path().join("documents.bin").is_file());
    assert!(storage.path().join("postings.bin").is_file());
    assert!(storage.path().join("lookup.bin").is_file());
}
