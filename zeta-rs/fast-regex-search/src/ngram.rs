use std::collections::BTreeSet;

const BYTE_PAIR_COUNT: usize = 1 << 16;
const MAX_NGRAM_BYTES: usize = 32;

/// Repository-local byte-pair frequencies used to prefer rare boundaries.
///
/// Counts are frozen for one index generation so indexing and querying always use the same
/// ordering. A rebuild retrains the weights from the current repository contents.
pub(crate) struct PairWeights {
    counts: Box<[u32; BYTE_PAIR_COUNT]>,
}

impl PairWeights {
    pub(crate) fn trained<'a>(documents: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut counts: Box<[u32; BYTE_PAIR_COUNT]> = Box::new([0; BYTE_PAIR_COUNT]);
        for bytes in documents {
            for pair in bytes.windows(2) {
                let index = pair_index(pair[0], pair[1]);
                counts[index] = counts[index].saturating_add(1);
            }
        }
        Self { counts }
    }

    pub(crate) fn from_counts(counts: Box<[u32; BYTE_PAIR_COUNT]>) -> Self {
        Self { counts }
    }

    pub(crate) fn counts(&self) -> &[u32; BYTE_PAIR_COUNT] {
        &self.counts
    }

    fn weight(&self, left: u8, right: u8) -> u64 {
        let frequency = self.counts[pair_index(left, right)];
        let rarity = u64::from(u32::MAX - frequency);
        (rarity << 32) | (hash_bytes(&[left, right]) & u64::from(u32::MAX))
    }
}

impl Default for PairWeights {
    fn default() -> Self {
        Self {
            counts: Box::new([0; BYTE_PAIR_COUNT]),
        }
    }
}

pub(crate) fn sparse_ngrams(bytes: &[u8], weights: &PairWeights) -> Vec<u64> {
    sparse_gram_spans(bytes, weights)
        .into_iter()
        .map(|gram| gram.hash)
        .collect()
}

/// Returns a small set of sparse grams whose byte ranges cover the required literal.
///
/// Every returned gram is also emitted by [`sparse_ngrams`] when the literal occurs inside a
/// larger document. Intersecting their posting lists can therefore remove false candidates but
/// cannot remove a real match.
pub(crate) fn covering_ngrams(bytes: &[u8], weights: &PairWeights) -> Vec<u64> {
    let spans = sparse_gram_spans(bytes, weights);
    if spans.is_empty() {
        return Vec::new();
    }
    let mut selected = BTreeSet::new();
    let mut covered_until = 0usize;
    while covered_until < bytes.len() {
        let best = spans
            .iter()
            .filter(|gram| gram.start <= covered_until && gram.end > covered_until)
            .max_by_key(|gram| (gram.end, usize::MAX - gram.start));
        let Some(best) = best else {
            break;
        };
        selected.insert(best.hash);
        covered_until = best.end;
    }
    selected.into_iter().collect()
}

#[derive(Clone, Copy)]
struct SparseGram {
    start: usize,
    end: usize,
    hash: u64,
}

fn sparse_gram_spans(bytes: &[u8], weights: &PairWeights) -> Vec<SparseGram> {
    if bytes.len() < 3 {
        return Vec::new();
    }
    let pair_weights = bytes
        .windows(2)
        .map(|pair| weights.weight(pair[0], pair[1]))
        .collect::<Vec<_>>();
    let mut grams = Vec::new();
    for left in 0..pair_weights.len().saturating_sub(1) {
        let mut internal_max = 0u64;
        let right_limit = (left + MAX_NGRAM_BYTES - 2).min(pair_weights.len() - 1);
        for right in left + 1..=right_limit {
            if right > left + 1 {
                internal_max = internal_max.max(pair_weights[right - 1]);
            }
            if pair_weights[left] > internal_max && pair_weights[right] > internal_max {
                grams.push(SparseGram {
                    start: left,
                    end: right + 2,
                    hash: hash_bytes(&bytes[left..right + 2]),
                });
            }
        }
    }
    grams.sort_by_key(|gram| gram.hash);
    grams.dedup_by_key(|gram| gram.hash);
    grams
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn pair_index(left: u8, right: u8) -> usize {
    (usize::from(left) << 8) | usize::from(right)
}

#[cfg(test)]
#[path = "ngram_tests.rs"]
mod tests;
