use super::BIGRAM_FREQUENCY_ORDER;
use super::BIGRAM_PAIR_COUNT;
use super::BIGRAM_RANKS;
use super::covering_ngrams;
use super::pair_index;
use super::pair_weight;
use super::sparse_ngrams;
use sha2::Digest;
use sha2::Sha256;

#[test]
fn version_one_frequency_order_has_the_reviewed_digest() {
    assert_eq!(
        format!("{:x}", Sha256::digest(BIGRAM_FREQUENCY_ORDER)),
        "97c07c74fb0947242a253597db342e1e3e0734dc3a7dde351a0a56fb53686919"
    );
}

#[test]
fn version_one_frequency_order_expands_to_a_rank_per_byte_pair() {
    let mut sorted = BIGRAM_RANKS.to_vec();
    sorted.sort_unstable();

    assert_eq!(sorted.len(), BIGRAM_PAIR_COUNT);
    assert_eq!(sorted[0], 0);
    assert_eq!(sorted[BIGRAM_PAIR_COUNT - 1], u16::MAX);
    assert!(sorted.windows(2).all(|ranks| ranks[1] == ranks[0] + 1));
}

#[test]
fn version_one_frequency_order_starts_with_expected_common_pairs() {
    assert_eq!(BIGRAM_RANKS[pair_index(b' ', b' ')], 0);
    assert_eq!(BIGRAM_RANKS[pair_index(b'\n', b' ')], 1);
    assert_eq!(BIGRAM_RANKS[pair_index(b'i', b'n')], 2);
    assert_eq!(BIGRAM_RANKS[pair_index(b'e', b'r')], 3);
}

#[test]
fn learned_ranks_are_limited_to_ascii_pairs() {
    assert_eq!(pair_weight(b'i', b'n'), 2);
    assert_eq!(
        pair_weight(0xc3, 0xa9),
        (pair_index(0xc3, 0xa9) as u64) << 16
    );
}

#[test]
fn sparse_ngrams_are_deterministic_and_variable_length() {
    let first = sparse_ngrams(b"authentication_token");
    let second = sparse_ngrams(b"authentication_token");
    assert_eq!(first, second);
    assert!(!first.is_empty());
}

#[test]
fn every_covering_gram_is_present_when_the_literal_is_embedded() {
    let document = b"prefix::authentication_token::suffix";
    let literal = b"authentication_token";
    let indexed = sparse_ngrams(document);
    let covering = covering_ngrams(literal);

    assert!(!covering.is_empty());
    assert!(covering.iter().all(|gram| indexed.contains(gram)));
}

#[test]
fn covering_grams_are_fewer_than_all_sparse_grams_for_long_literals() {
    let literal = b"request_authentication_token_handler";
    let all = sparse_ngrams(literal);
    let covering = covering_ngrams(literal);

    assert!(!covering.is_empty());
    assert!(covering.len() < all.len());
}

#[test]
fn covering_grams_never_create_false_negatives_for_any_embedded_slice() {
    let document = b"fn parse_request_authentication_token(input_42: &str) -> Result<()> { ok() }";
    let indexed = sparse_ngrams(document);

    for start in 0..document.len() - 2 {
        for end in start + 3..=document.len().min(start + 32) {
            let covering = covering_ngrams(&document[start..end]);
            assert!(
                covering.iter().all(|gram| indexed.contains(gram)),
                "missing gram for byte range {start}..{end}"
            );
        }
    }
}
