use super::*;
use zeta_content_search::ContentSearchMatch;
use zeta_content_search::ContentSearchMatchRange;

#[test]
fn maps_protocol_query_to_content_search_query() {
    let query = search_query(ContentSearchStartParams {
        dir_id: None,
        session_directory: None,
        query: "needle".into(),
        pattern_kind: ContentSearchPatternKind::Regex,
        case_sensitivity: ContentSearchProtocolCaseSensitivity::Insensitive,
        include_patterns: vec!["src/**".into()],
        exclude_patterns: vec!["**/*.test.rs".into()],
        max_results: 400,
    });

    assert_eq!(
        query,
        ContentSearchQuery {
            query: "needle".into(),
            pattern: ContentSearchPattern::Regex,
            case_sensitivity: ContentSearchCaseSensitivity::Insensitive,
            include_patterns: vec!["src/**".into()],
            exclude_patterns: vec!["**/*.test.rs".into()],
            max_results: 400,
        }
    );
}

#[test]
fn maps_content_search_page_to_protocol_result() {
    let result = search_page(
        "search-1".into(),
        ContentSearchPage {
            matches: vec![ContentSearchMatch {
                path: "src/lib.rs".into(),
                line_number: 7,
                preview: "let needle = true;".into(),
                ranges: vec![ContentSearchMatchRange { start: 4, end: 10 }],
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
        [ContentSearchProtocolMatchRange { start: 4, end: 10 }]
    );
    assert_eq!(result.next_match, 1);
    assert!(result.completed);
    assert!(!result.limit_hit);
    assert_eq!(result.error, None);
}
