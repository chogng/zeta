use super::*;
use zeta_workspace_search::WorkspaceSearchMatch;
use zeta_workspace_search::WorkspaceSearchMatchRange;

#[test]
fn maps_protocol_query_to_workspace_search_query() {
    let query = search_query(WorkspaceSearchStartParams {
        workspace_folder_id: None,
        session_directory: None,
        query: "needle".into(),
        pattern_kind: WorkspaceSearchPatternKind::Regex,
        case_sensitivity: WorkspaceSearchProtocolCaseSensitivity::Insensitive,
        include_patterns: vec!["src/**".into()],
        exclude_patterns: vec!["**/*.test.rs".into()],
        max_results: 400,
    });

    assert_eq!(
        query,
        WorkspaceSearchQuery {
            query: "needle".into(),
            pattern: WorkspaceSearchPattern::Regex,
            case_sensitivity: WorkspaceSearchCaseSensitivity::Insensitive,
            include_patterns: vec!["src/**".into()],
            exclude_patterns: vec!["**/*.test.rs".into()],
            max_results: 400,
        }
    );
}

#[test]
fn maps_workspace_search_page_to_protocol_result() {
    let result = search_page(
        "search-1".into(),
        WorkspaceSearchPage {
            matches: vec![WorkspaceSearchMatch {
                path: "src/lib.rs".into(),
                line_number: 7,
                preview: "let needle = true;".into(),
                ranges: vec![WorkspaceSearchMatchRange { start: 4, end: 10 }],
            }],
            next_match: 1,
            completed: true,
            limit_hit: false,
            error: None,
        },
    );

    assert_eq!(result.search_id, "search-1");
    assert_eq!(result.matches.len(), 1);
    assert_eq!(
        result.matches[0].path,
        std::path::PathBuf::from("src/lib.rs")
    );
    assert_eq!(
        result.matches[0].ranges,
        [WorkspaceSearchProtocolMatchRange { start: 4, end: 10 }]
    );
    assert_eq!(result.next_match, 1);
    assert!(result.completed);
    assert!(!result.limit_hit);
    assert_eq!(result.error, None);
}
