use super::ParsedShellToken;
use super::parse_query_into_tokens;
use super::parse_shell_tokens;

#[test]
fn sentence_parser_matches_the_classifier_token_contract() {
    assert_eq!(
        parse_query_into_tokens("This is a question?"),
        ["This", "is", "a", "question"]
    );
    assert_eq!(
        parse_query_into_tokens("A quote \"Inside 'something' quote\""),
        ["A", "quote", "\"Inside 'something' quote\""]
    );
    assert_eq!(
        parse_query_into_tokens("Empty quote \"\"!?!"),
        ["Empty", "quote"]
    );
    assert_eq!(
        parse_query_into_tokens("www.google.com"),
        ["www.google.com"]
    );
    assert_eq!(
        parse_query_into_tokens("Command `mockery --name example_interface`"),
        ["Command", "`mockery --name example_interface`"]
    );
}

#[test]
fn shell_parser_preserves_quotes_and_resets_command_indices() {
    assert_eq!(
        parse_shell_tokens("git commit -m 'hello world' && cargo test"),
        [
            ParsedShellToken {
                text: "git".to_owned(),
                token_index: 0,
            },
            ParsedShellToken {
                text: "commit".to_owned(),
                token_index: 1,
            },
            ParsedShellToken {
                text: "-m".to_owned(),
                token_index: 2,
            },
            ParsedShellToken {
                text: "'hello world'".to_owned(),
                token_index: 3,
            },
            ParsedShellToken {
                text: "cargo".to_owned(),
                token_index: 0,
            },
            ParsedShellToken {
                text: "test".to_owned(),
                token_index: 1,
            },
        ]
    );
}

#[test]
fn shell_parser_splits_flag_values_without_advancing_the_argument_index() {
    assert_eq!(
        parse_shell_tokens("cargo test --package=zeta"),
        [
            ParsedShellToken {
                text: "cargo".to_owned(),
                token_index: 0,
            },
            ParsedShellToken {
                text: "test".to_owned(),
                token_index: 1,
            },
            ParsedShellToken {
                text: "--package".to_owned(),
                token_index: 2,
            },
            ParsedShellToken {
                text: "zeta".to_owned(),
                token_index: 2,
            },
        ]
    );
}
