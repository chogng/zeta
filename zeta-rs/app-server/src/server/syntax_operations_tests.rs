use super::*;

fn open_params(owner: u64, text: &str) -> (SyntaxAnalysisService, SyntaxTokenSnapshotDto) {
    let service = SyntaxAnalysisService::new();
    let snapshot = service
        .open(
            owner,
            SyntaxOpenParams {
                document_id: "model-1".into(),
                document_uri: "file:///workspace/main.rs".into(),
                language: SyntaxLanguageDto::Rust,
                revision: 1,
                text: text.into(),
            },
        )
        .expect("Rust document should open");
    (service, snapshot)
}

#[test]
fn syntax_session_applies_utf16_batch_and_advances_one_revision() {
    let owner = 7;
    let (service, initial) = open_params(owner, "fn 😀first() {}\nfn second() {}\n");
    assert_eq!(initial.revision, 1);
    assert_eq!(initial.data.len() % 5, 0);

    let changed = service
        .change(
            owner,
            SyntaxChangeParams {
                document_id: "model-1".into(),
                previous_revision: 1,
                revision: 2,
                edits: vec![
                    SyntaxTextEditDto {
                        start_utf16: 5,
                        end_utf16: 10,
                        text: "one".into(),
                    },
                    SyntaxTextEditDto {
                        start_utf16: 19,
                        end_utf16: 25,
                        text: "two".into(),
                    },
                ],
            },
        )
        .expect("UTF-16 batch should apply");

    assert_eq!(changed.revision, 2);
    assert_eq!(changed.result_id, "2");
    assert_eq!(changed.data.len() % 5, 0);
}

#[test]
fn syntax_sessions_enforce_connection_owner_and_revision() {
    let owner = 11;
    let (service, _) = open_params(owner, "fn main() {}\n");
    let params = SyntaxChangeParams {
        document_id: "model-1".into(),
        previous_revision: 1,
        revision: 2,
        edits: vec![SyntaxTextEditDto {
            start_utf16: 3,
            end_utf16: 7,
            text: "run".into(),
        }],
    };

    assert_eq!(
        service.change(owner + 1, params.clone()),
        Err(SyntaxAnalysisError::NotOpen),
    );
    service.change(owner, params).expect("owner should update");
    assert_eq!(
        service.change(
            owner,
            SyntaxChangeParams {
                document_id: "model-1".into(),
                previous_revision: 1,
                revision: 3,
                edits: vec![SyntaxTextEditDto {
                    start_utf16: 0,
                    end_utf16: 0,
                    text: "pub ".into(),
                }],
            },
        ),
        Err(SyntaxAnalysisError::RevisionMismatch),
    );
}

#[test]
fn semantic_token_encoding_is_relative_and_non_overlapping() {
    let (_, snapshot) = open_params(3, "pub fn main() { let value = 42; }\n");
    let mut line = 0u32;
    let mut start = 0u32;
    let mut previous_end = 0u32;

    for token in snapshot.data.chunks_exact(5) {
        line += token[0];
        start = if token[0] == 0 {
            start + token[1]
        } else {
            token[1]
        };
        if token[0] == 0 {
            assert!(start >= previous_end);
        }
        assert!(token[2] > 0);
        assert!(token[3] <= 13);
        assert_eq!(token[4], 0);
        previous_end = start + token[2];
    }
    assert_eq!(line, 0);
}

#[test]
fn syntax_sessions_support_json_and_jsonc_languages() {
    let service = SyntaxAnalysisService::new();
    for (index, (language, uri, text, expected_token_type)) in [
        (
            SyntaxLanguageDto::Json,
            "file:///workspace/settings.json",
            "{\"enabled\":true}",
            2,
        ),
        (
            SyntaxLanguageDto::Jsonc,
            "file:///workspace/settings.jsonc",
            "{// local\n\"enabled\":true}",
            1,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let snapshot = service
            .open(
                19,
                SyntaxOpenParams {
                    document_id: format!("model-{index}"),
                    document_uri: uri.into(),
                    language,
                    revision: 1,
                    text: text.into(),
                },
            )
            .expect("JSON-family syntax document should open");
        assert_eq!(snapshot.revision, 1);
        assert!(!snapshot.data.is_empty());
        assert!(
            snapshot
                .data
                .chunks_exact(5)
                .any(|token| token[3] == expected_token_type)
        );
    }
}
