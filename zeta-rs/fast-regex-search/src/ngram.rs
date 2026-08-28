use std::collections::BTreeSet;

const MAX_NGRAM_BYTES: usize = 32;

pub(crate) fn sparse_ngrams(bytes: &[u8]) -> Vec<u64> {
    if bytes.len() < 3 {
        return Vec::new();
    }
    let weights = bytes
        .windows(2)
        .map(|pair| pair_weight(pair[0], pair[1]))
        .collect::<Vec<_>>();
    let mut grams = BTreeSet::new();
    for left in 0..weights.len().saturating_sub(1) {
        let mut internal_max = 0u64;
        let right_limit = (left + MAX_NGRAM_BYTES - 1).min(weights.len() - 1);
        for right in left + 1..=right_limit {
            if right > left + 1 {
                internal_max = internal_max.max(weights[right - 1]);
            }
            if weights[left] > internal_max && weights[right] > internal_max {
                grams.insert(hash_bytes(&bytes[left..right + 2]));
            }
        }
    }
    grams.into_iter().collect()
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn pair_weight(left: u8, right: u8) -> u64 {
    let pair = [left, right];
    hash_bytes(&pair)
}

#[cfg(test)]
#[path = "ngram_tests.rs"]
mod tests;
