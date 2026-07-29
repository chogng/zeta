use super::*;
use serde_json::json;
use std::io::Cursor;

#[test]
fn parses_utf8_match_ranges_as_utf16_offsets() {
    let line = json!({
        "type": "match",
        "data": {
            "path": {"text": "./src/lib.rs"},
            "lines": {"text": "let 搜索 = \"needle\";\n"},
            "line_number": 7,
            "submatches": [{
                "match": {"text": "needle"},
                "start": 14,
                "end": 20
            }]
        }
    })
    .to_string();

    let parsed = parse_match(&line).unwrap().unwrap();

    assert_eq!(parsed.path, Path::new("src/lib.rs"));
    assert_eq!(parsed.line_number, 7);
    assert_eq!(parsed.preview, "let 搜索 = \"needle\";");
    assert_eq!(
        parsed.ranges,
        [WorkspaceSearchMatchRange { start: 10, end: 16 }]
    );
}

#[test]
fn builds_typed_ripgrep_arguments_without_shell_parsing() {
    let arguments = search_arguments(&WorkspaceSearchStartParams {
        query: "needle".into(),
        pattern_kind: WorkspaceSearchPatternKind::Literal,
        case_sensitivity: WorkspaceSearchCaseSensitivity::Insensitive,
        include_patterns: vec!["src/**".into()],
        exclude_patterns: vec!["**/*.test.ts".into()],
        max_results: 20,
    });

    assert!(arguments.windows(2).any(|pair| pair == ["-g", "src/**"]));
    assert!(
        arguments
            .windows(2)
            .any(|pair| pair == ["-g", "!**/*.test.ts"])
    );
    assert!(arguments.contains(&"--fixed-strings".into()));
    assert_eq!(&arguments[arguments.len() - 3..], ["--", "needle", "."]);
}

#[test]
fn rejects_globs_that_can_invert_or_escape_the_typed_filter() {
    for pattern in ["!src/**", "../outside", "/absolute", "C:/absolute"] {
        assert_eq!(
            validate_glob(pattern),
            Err(WorkspaceSearchError::InvalidInput)
        );
    }
}

#[test]
fn result_limit_is_reported_only_when_an_additional_match_exists() {
    let match_line = json!({
        "type": "match",
        "data": {
            "path": {"text": "./src/lib.rs"},
            "lines": {"text": "needle\n"},
            "line_number": 1,
            "submatches": [{
                "match": {"text": "needle"},
                "start": 0,
                "end": 6
            }]
        }
    })
    .to_string();

    let exact_state = Arc::new(Mutex::new(SearchJobState::default()));
    let exact_cancellation = CancellationSource::new();
    parse_stdout(
        Cursor::new(format!("{match_line}\n")),
        1,
        &exact_cancellation,
        &exact_state,
    )
    .unwrap();
    assert_eq!(exact_state.lock().unwrap().matches.len(), 1);
    assert!(!exact_state.lock().unwrap().limit_hit);

    let overflow_state = Arc::new(Mutex::new(SearchJobState::default()));
    let overflow_cancellation = CancellationSource::new();
    parse_stdout(
        Cursor::new(format!("{match_line}\n{match_line}\n")),
        1,
        &overflow_cancellation,
        &overflow_state,
    )
    .unwrap();
    assert_eq!(overflow_state.lock().unwrap().matches.len(), 1);
    assert!(overflow_state.lock().unwrap().limit_hit);
    assert!(overflow_cancellation.token().is_cancelled());
}
