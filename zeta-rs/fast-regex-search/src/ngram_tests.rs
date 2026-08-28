use super::PairWeights;
use super::covering_ngrams;
use super::sparse_ngrams;

#[test]
fn sparse_ngrams_are_deterministic_and_variable_length() {
    let weights = PairWeights::trained([b"authentication_token".as_slice()]);
    let first = sparse_ngrams(b"authentication_token", &weights);
    let second = sparse_ngrams(b"authentication_token", &weights);
    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn every_covering_gram_is_present_when_the_literal_is_embedded() {
    let document = b"prefix::authentication_token::suffix";
    let literal = b"authentication_token";
    let weights = PairWeights::trained([document.as_slice()]);
    let indexed = sparse_ngrams(document, &weights);
    let covering = covering_ngrams(literal, &weights);

    assert!(!covering.is_empty());
    assert!(covering.iter().all(|gram| indexed.contains(gram)));
}

#[test]
fn covering_grams_are_fewer_than_all_sparse_grams_for_long_literals() {
    let literal = b"workspace_authentication_token_handler";
    let weights = PairWeights::trained([literal.as_slice()]);

    let all = sparse_ngrams(literal, &weights);
    let covering = covering_ngrams(literal, &weights);

    assert!(!covering.is_empty());
    assert!(covering.len() < all.len());
}

#[test]
fn covering_grams_never_create_false_negatives_for_any_embedded_slice() {
    let document =
        b"fn parse_workspace_authentication_token(input_42: &str) -> Result<()> { ok() }";
    let weights = PairWeights::trained([document.as_slice()]);
    let indexed = sparse_ngrams(document, &weights);

    for start in 0..document.len() - 2 {
        for end in start + 3..=document.len().min(start + 32) {
            let covering = covering_ngrams(&document[start..end], &weights);
            assert!(
                covering.iter().all(|gram| indexed.contains(gram)),
                "missing gram for byte range {start}..{end}"
            );
        }
    }
}
