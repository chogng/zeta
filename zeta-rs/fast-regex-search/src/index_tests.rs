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
        scope: PathBuf::new(),
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
    fs::write(
        directory.path().join("source.rs"),
        "persistent_marker\nPERSISTENT_MARKER\n",
    )
    .expect("source");
    let storage = tempfile::tempdir().expect("storage");
    let storage_mode = FastRegexSearchStorage::Persistent(storage.path().to_path_buf());
    let index = search(&directory, storage_mode.clone());

    let built = index.rebuild().expect("rebuild");
    drop(index);
    let reopened = search(&directory, storage_mode);
    let mut insensitive = query("persistent_marker");
    insensitive.case_sensitivity = FastRegexCaseSensitivity::Insensitive;

    assert_eq!(reopened.snapshot(), built);
    assert_eq!(reopened.search(&insensitive).unwrap().matches.len(), 2);
    assert!(storage.path().join("complete.bin").is_file());
    assert!(storage.path().join("documents.bin").is_file());
    assert!(storage.path().join("postings.bin").is_file());
    assert!(storage.path().join("lookup.bin").is_file());
    assert!(storage.path().join("weights.bin").is_file());
}

#[test]
fn changed_workspace_invalidates_a_persisted_generation_before_search() {
    let directory = workspace();
    let source = directory.path().join("source.rs");
    fs::write(&source, "before_persisted_marker\n").expect("source");
    let storage = tempfile::tempdir().expect("storage");
    let storage_mode = FastRegexSearchStorage::Persistent(storage.path().to_path_buf());
    let index = search(&directory, storage_mode.clone());
    index.rebuild().expect("rebuild");
    drop(index);
    fs::write(&source, "after_persisted_marker\n").expect("changed source");

    let reopened = search(&directory, storage_mode);

    assert_eq!(reopened.snapshot().generation, 0);
    assert!(matches!(
        reopened.search(&query("after_persisted_marker")),
        Err(FastRegexError::NotReady)
    ));
}

#[test]
fn persisted_change_layer_restores_updates_and_deletions_without_rewriting_the_base() {
    let directory = workspace();
    let changed = directory.path().join("changed.rs");
    let deleted = directory.path().join("deleted.rs");
    let later_deleted = directory.path().join("later-deleted.rs");
    fs::write(&changed, "before_layer_marker\n").expect("changed source");
    fs::write(&deleted, "removed_first_marker\n").expect("deleted source");
    fs::write(&later_deleted, "removed_later_marker\n").expect("later deleted source");
    let storage = tempfile::tempdir().expect("storage");
    let storage_mode = FastRegexSearchStorage::Persistent(storage.path().to_path_buf());
    let index = search(&directory, storage_mode.clone());
    index.rebuild().expect("rebuild");
    let base_lookup_size = fs::metadata(storage.path().join("lookup.bin"))
        .expect("lookup metadata")
        .len();
    fs::write(&changed, "after_layer_marker\n").expect("changed source update");
    fs::remove_file(&deleted).expect("delete source");

    let outcome = index
        .refresh_observed_paths(&[changed.clone(), deleted])
        .expect("layer update");
    let published = match outcome {
        FastRegexUpdateOutcome::Published(snapshot) => snapshot,
        other => panic!("expected published update, got {other:?}"),
    };
    assert_eq!(
        fs::metadata(storage.path().join("lookup.bin"))
            .expect("lookup metadata after update")
            .len(),
        base_lookup_size
    );
    assert!(
        fs::metadata(storage.path().join("delta.bin"))
            .expect("delta metadata")
            .len()
            < base_lookup_size
    );
    drop(index);

    let reopened = search(&directory, storage_mode.clone());

    assert_eq!(reopened.snapshot(), published);
    assert_eq!(
        reopened
            .search(&query("after_layer_marker"))
            .unwrap()
            .matches
            .len(),
        1
    );
    assert_eq!(
        reopened
            .search(&query("before_layer_marker"))
            .unwrap()
            .matches
            .len(),
        0
    );
    assert_eq!(
        reopened
            .search(&query("removed_first_marker"))
            .unwrap()
            .matches
            .len(),
        0
    );
    fs::write(&changed, "final_layer_marker\n").expect("second update");
    fs::remove_file(&later_deleted).expect("later deletion");
    let second = reopened
        .refresh_observed_paths(&[changed, later_deleted])
        .expect("second layer update");
    let second_snapshot = match second {
        FastRegexUpdateOutcome::Published(snapshot) => snapshot,
        other => panic!("expected second published update, got {other:?}"),
    };
    drop(reopened);

    let reopened_again = search(&directory, storage_mode);

    assert_eq!(reopened_again.snapshot(), second_snapshot);
    assert_eq!(
        reopened_again
            .search(&query("final_layer_marker"))
            .unwrap()
            .matches
            .len(),
        1
    );
    assert_eq!(
        reopened_again
            .search(&query("removed_later_marker"))
            .unwrap()
            .matches
            .len(),
        0
    );
}

#[test]
fn corrupt_completed_generation_is_rejected_instead_of_serving_partial_postings() {
    let directory = workspace();
    fs::write(directory.path().join("source.rs"), "corrupt_marker\n").expect("source");
    let storage = tempfile::tempdir().expect("storage");
    let storage_mode = FastRegexSearchStorage::Persistent(storage.path().to_path_buf());
    let index = search(&directory, storage_mode.clone());
    index.rebuild().expect("rebuild");
    drop(index);
    fs::write(storage.path().join("lookup.bin"), b"truncated").expect("corrupt lookup");

    let result = FastRegexSearch::open(
        WorkspaceRoot::open(directory.path()).expect("root"),
        storage_mode,
        FastRegexSearchLimits::default(),
    );

    assert!(matches!(result, Err(FastRegexError::CorruptIndex(_))));
}

#[test]
fn scope_and_filename_glob_are_applied_independently() {
    let directory = workspace();
    fs::create_dir_all(directory.path().join("src/nested")).expect("source directories");
    fs::write(directory.path().join("src/root.rs"), "scoped_marker\n").expect("root source");
    fs::write(
        directory.path().join("src/nested/child.rs"),
        "scoped_marker\n",
    )
    .expect("nested source");
    fs::write(
        directory.path().join("src/nested/child.txt"),
        "scoped_marker\n",
    )
    .expect("non-matching extension");
    fs::write(directory.path().join("outside.rs"), "scoped_marker\n").expect("outside source");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let mut scoped = query("scoped_marker");
    scoped.scope = PathBuf::from("src");
    scoped.include_patterns = vec!["*.rs".into()];

    let result = index.search(&scoped).expect("scoped search");

    assert_eq!(
        result
            .matches
            .iter()
            .map(|found| found.path.as_path())
            .collect::<Vec<_>>(),
        [Path::new("src/nested/child.rs"), Path::new("src/root.rs")]
    );
}

#[test]
fn prefix_and_suffix_postings_are_intersected_before_source_scans() {
    let directory = workspace();
    for index in 0..100 {
        fs::write(
            directory.path().join(format!("prefix-{index}.txt")),
            "authentication_noise_only\n",
        )
        .expect("prefix noise");
        fs::write(
            directory.path().join(format!("suffix-{index}.txt")),
            "unrelated_token\n",
        )
        .expect("suffix noise");
    }
    fs::write(
        directory.path().join("match.txt"),
        "authentication_value_token\n",
    )
    .expect("match");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let result = index
        .search(&query(r"authentication_.*token"))
        .expect("search");

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.statistics.indexed_file_count, 201);
    assert_eq!(result.statistics.candidate_file_count, 1);
    assert_eq!(result.statistics.scanned_file_count, 1);
}

#[test]
fn case_modes_match_ripgrep_style_smart_case() {
    let directory = workspace();
    fs::write(
        directory.path().join("cases.txt"),
        "lower_marker\nLOWER_MARKER\nMixed_Marker\n",
    )
    .expect("cases");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let mut sensitive = query("lower_marker");
    sensitive.case_sensitivity = FastRegexCaseSensitivity::Sensitive;
    let mut insensitive = sensitive.clone();
    insensitive.case_sensitivity = FastRegexCaseSensitivity::Insensitive;
    let mut smart_lower = sensitive.clone();
    smart_lower.case_sensitivity = FastRegexCaseSensitivity::Smart;
    let mut smart_upper = query("LOWER_MARKER");
    smart_upper.case_sensitivity = FastRegexCaseSensitivity::Smart;

    assert_eq!(index.search(&sensitive).unwrap().matches.len(), 1);
    assert_eq!(index.search(&insensitive).unwrap().matches.len(), 2);
    assert_eq!(index.search(&smart_lower).unwrap().matches.len(), 2);
    assert_eq!(index.search(&smart_upper).unwrap().matches.len(), 1);
}

#[test]
fn literal_patterns_escape_regex_metacharacters() {
    let directory = workspace();
    fs::write(
        directory.path().join("literal.txt"),
        "call(foo)\ncallXfooY\n",
    )
    .expect("literal");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let mut literal = query("call(foo)");
    literal.pattern = FastRegexPattern::Literal;

    let result = index.search(&literal).expect("literal search");

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].preview, "call(foo)");
}

#[test]
fn line_results_keep_all_ranges_and_preserve_searchable_carriage_returns() {
    let directory = workspace();
    fs::write(
        directory.path().join("lines.txt"),
        "hit hit\r\nmiss\r\nhit\r\n",
    )
    .expect("lines");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let result = index.search(&query("hit")).expect("search");

    assert_eq!(
        result.matches,
        [
            FastRegexMatch {
                path: PathBuf::from("lines.txt"),
                line_number: 1,
                preview: "hit hit\r".into(),
                ranges: vec![
                    FastRegexRange {
                        start_byte: 0,
                        end_byte: 3,
                    },
                    FastRegexRange {
                        start_byte: 4,
                        end_byte: 7,
                    },
                ],
            },
            FastRegexMatch {
                path: PathBuf::from("lines.txt"),
                line_number: 3,
                preview: "hit\r".into(),
                ranges: vec![FastRegexRange {
                    start_byte: 0,
                    end_byte: 3,
                }],
            },
        ]
    );
}

#[test]
fn line_terminators_match_ripgrep_without_a_phantom_trailing_line() {
    let directory = workspace();
    fs::write(directory.path().join("lf.txt"), "value\n").expect("lf source");
    fs::write(directory.path().join("crlf.txt"), "value\r\n").expect("crlf source");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");

    let empty_line = index.search(&query(r"^$")).expect("empty line search");
    let carriage_return = index
        .search(&query(r"\r$"))
        .expect("carriage return search");

    assert!(empty_line.matches.is_empty());
    assert_eq!(carriage_return.matches.len(), 1);
    assert_eq!(carriage_return.matches[0].path, Path::new("crlf.txt"));
    assert_eq!(
        carriage_return.matches[0].ranges,
        [FastRegexRange {
            start_byte: 5,
            end_byte: 6,
        }]
    );
}

#[test]
fn result_limit_reports_only_when_an_additional_matching_line_exists() {
    let directory = workspace();
    fs::write(
        directory.path().join("many.txt"),
        "marker\nmarker\nmarker\n",
    )
    .expect("many");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let mut limited = query("marker");
    limited.max_results = 2;

    let result = index.search(&limited).expect("limited search");

    assert_eq!(result.matches.len(), 2);
    assert!(result.limit_hit);
}

#[test]
fn workspace_scan_matches_default_hidden_and_gitignore_boundaries() {
    let directory = workspace();
    fs::write(directory.path().join(".gitignore"), "ignored.txt\n").expect("ignore file");
    fs::write(directory.path().join("visible.txt"), "boundary_marker\n").expect("visible");
    fs::write(directory.path().join("ignored.txt"), "boundary_marker\n").expect("ignored");
    fs::write(directory.path().join(".hidden.txt"), "boundary_marker\n").expect("hidden");
    let index = search(&directory, FastRegexSearchStorage::Memory);

    let snapshot = index.rebuild().expect("rebuild");
    let result = index.search(&query("boundary_marker")).expect("search");

    assert_eq!(snapshot.indexed_file_count, 1);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].path, Path::new("visible.txt"));
}

#[test]
fn incremental_refresh_adds_and_removes_files_without_rebuilding() {
    let directory = workspace();
    fs::write(directory.path().join("base.txt"), "base_marker\n").expect("base");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let added = directory.path().join("added.txt");
    fs::write(&added, "added_marker\n").expect("added");

    let added_outcome = index
        .refresh_observed_paths(std::slice::from_ref(&added))
        .expect("add");
    assert!(matches!(
        added_outcome,
        FastRegexUpdateOutcome::Published(_)
    ));
    assert_eq!(
        index.search(&query("added_marker")).unwrap().matches.len(),
        1
    );

    fs::remove_file(&added).expect("remove");
    let removed_outcome = index
        .refresh_observed_paths(&[added])
        .expect("remove refresh");
    assert!(matches!(
        removed_outcome,
        FastRegexUpdateOutcome::Published(_)
    ));
    assert_eq!(
        index.search(&query("added_marker")).unwrap().matches.len(),
        0
    );
}

#[test]
fn incremental_refresh_does_not_publish_gitignored_new_files() {
    let directory = workspace();
    fs::write(directory.path().join(".gitignore"), "ignored-new.txt\n").expect("ignore file");
    fs::write(directory.path().join("base.txt"), "base_marker\n").expect("base");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let ignored = directory.path().join("ignored-new.txt");
    fs::write(&ignored, "ignored_new_marker\n").expect("ignored source");

    let outcome = index.refresh_observed_paths(&[ignored]).expect("refresh");

    assert_eq!(outcome, FastRegexUpdateOutcome::NoChange);
    assert_eq!(
        index
            .search(&query("ignored_new_marker"))
            .unwrap()
            .matches
            .len(),
        0
    );
}

#[test]
fn changing_git_info_exclude_rebuilds_the_ignore_matcher_and_index() {
    let directory = workspace();
    fs::create_dir_all(directory.path().join(".git/info")).expect("git info directory");
    let exclude = directory.path().join(".git/info/exclude");
    fs::write(&exclude, "excluded-by-info.txt\n").expect("git exclude");
    fs::write(
        directory.path().join("excluded-by-info.txt"),
        "git_info_marker\n",
    )
    .expect("excluded source");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("initial rebuild");
    assert!(
        index
            .search(&query("git_info_marker"))
            .unwrap()
            .matches
            .is_empty()
    );

    fs::write(&exclude, "").expect("clear git exclude");
    let outcome = index
        .refresh_observed_paths(&[exclude])
        .expect("refresh ignore control");

    assert!(matches!(outcome, FastRegexUpdateOutcome::Rebuilt(_)));
    assert_eq!(
        index
            .search(&query("git_info_marker"))
            .unwrap()
            .matches
            .len(),
        1
    );
}

#[test]
fn stale_candidate_source_is_rejected_until_the_watcher_refreshes_it() {
    let directory = workspace();
    let source = directory.path().join("source.txt");
    fs::write(&source, "old_stale_marker\n").expect("old source");
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    fs::write(&source, "new_stale_marker\n").expect("new source");

    let error = index.search(&query("old_stale_marker")).unwrap_err();

    assert!(matches!(error, FastRegexError::StaleSource(path) if path == Path::new("source.txt")));
}

#[test]
fn invalid_queries_and_storage_limits_fail_at_the_public_boundary() {
    let directory = workspace();
    let root = WorkspaceRoot::open(directory.path()).expect("root");
    let invalid_limits = FastRegexSearchLimits {
        max_files: 0,
        ..FastRegexSearchLimits::default()
    };
    assert!(matches!(
        FastRegexSearch::open(root, FastRegexSearchStorage::Memory, invalid_limits),
        Err(FastRegexError::InvalidLimits)
    ));

    let index = search(&directory, FastRegexSearchStorage::Memory);
    fs::write(directory.path().join("source.txt"), "text\n").expect("source");
    index.rebuild().expect("rebuild");
    let mut invalid_scope = query("text");
    invalid_scope.scope = PathBuf::from("../outside");
    assert!(matches!(
        index.search(&invalid_scope),
        Err(FastRegexError::InvalidQuery("search scope is invalid"))
    ));
    let mut invalid_glob = query("text");
    invalid_glob.include_patterns = vec!["[".into()];
    assert!(matches!(
        index.search(&invalid_glob),
        Err(FastRegexError::InvalidGlob)
    ));
    assert!(matches!(
        index.search(&query("(")),
        Err(FastRegexError::Regex(_))
    ));
}

#[test]
fn unsupported_text_files_are_skipped_without_aborting_the_generation() {
    let directory = workspace();
    fs::write(directory.path().join("valid.txt"), "valid_marker\n").expect("valid");
    fs::write(directory.path().join("binary.bin"), b"binary\0marker").expect("binary");
    fs::write(directory.path().join("invalid.txt"), [0xff, 0xfe, 0xfd]).expect("invalid utf8");
    let index = search(&directory, FastRegexSearchStorage::Memory);

    let snapshot = index.rebuild().expect("rebuild");

    assert_eq!(snapshot.indexed_file_count, 1);
    assert_eq!(
        index.search(&query("valid_marker")).unwrap().matches.len(),
        1
    );
}

#[test]
fn indexed_candidates_match_an_exhaustive_scan_across_regex_shapes() {
    let directory = workspace();
    let documents = [
        (
            "one.rs",
            "fn alpha_rare_handler() {}\nauthentication_value_token\nfoo_bar\n",
        ),
        (
            "two.rs",
            "fn beta_rare_handler() {}\nauthentication noise\ntoken only\nfoo_baz\n",
        ),
        (
            "three.rs",
            "fn gamma_handler() {}\nAUTHENTICATION_VALUE_TOKEN\nnumber 123\n",
        ),
        ("unicode.rs", "fn 解析器() {}\n认证_值_令牌\n"),
    ];
    for (path, content) in documents {
        fs::write(directory.path().join(path), content).expect("corpus source");
    }
    let index = search(&directory, FastRegexSearchStorage::Memory);
    index.rebuild().expect("rebuild");
    let cases = [
        (r"authentication_.*token", false),
        (r"(?:alpha|beta)_rare_handler", false),
        (r"(?:authentication|a).*token", false),
        (r"foo_(?:bar|baz)", false),
        (r"^fn\s+", false),
        (r"[0-9]{3}", false),
        (r"认证_.*令牌", false),
        (r"authentication_.*token", true),
        (r"missing_workspace_marker_.*suffix", false),
    ];

    for (pattern, insensitive) in cases {
        let mut indexed_query = query(pattern);
        if insensitive {
            indexed_query.case_sensitivity = FastRegexCaseSensitivity::Insensitive;
        }
        let actual = index.search(&indexed_query).expect("indexed search");
        let matcher = RegexBuilder::new(pattern)
            .case_insensitive(insensitive)
            .build()
            .expect("exhaustive matcher");
        let mut expected = Vec::new();
        let mut limit_hit = false;
        for (path, content) in documents {
            collect_matches(
                Path::new(path),
                content,
                &matcher,
                indexed_query.max_results,
                &mut expected,
                &mut limit_hit,
            );
        }
        expected.sort_by(|left, right| {
            (&left.path, left.line_number).cmp(&(&right.path, right.line_number))
        });

        assert_eq!(actual.matches, expected, "pattern {pattern}");
        assert!(!limit_hit);
    }
}
