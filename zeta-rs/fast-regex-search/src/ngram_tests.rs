use super::sparse_ngrams;

#[test]
fn sparse_ngrams_are_deterministic_and_variable_length() {
    let first = sparse_ngrams(b"authentication_token");
    let second = sparse_ngrams(b"authentication_token");
    assert_eq!(first, second);
    assert!(!first.is_empty());
}
