use super::{Edit, edits};
use crate::{DiffLimits, NeverCancel};

#[test]
fn exhaustive_small_sequences_produce_a_shortest_ordered_edit_script() {
    let sequences = binary_sequences(5);
    for old in &sequences {
        for new in &sequences {
            let script = edits(old, new, DiffLimits::default(), &NeverCancel).unwrap();
            assert_script_reconstructs(old, new, &script);
            assert_eq!(
                script
                    .iter()
                    .filter(|edit| !matches!(edit, Edit::Equal { .. }))
                    .count(),
                reference_edit_distance(old, new),
                "old={old:?}, new={new:?}, script={script:?}"
            );
        }
    }
}

fn binary_sequences(maximum_length: usize) -> Vec<Vec<u8>> {
    let mut sequences = Vec::new();
    for length in 0..=maximum_length {
        for bits in 0..(1usize << length) {
            sequences.push(
                (0..length)
                    .map(|position| ((bits >> position) & 1) as u8)
                    .collect(),
            );
        }
    }
    sequences
}

fn assert_script_reconstructs(old: &[u8], new: &[u8], script: &[Edit]) {
    let mut old_cursor = 0;
    let mut new_cursor = 0;
    for edit in script {
        match *edit {
            Edit::Equal {
                old: old_index,
                new: new_index,
            } => {
                assert_eq!((old_index, new_index), (old_cursor, new_cursor));
                assert_eq!(old[old_index], new[new_index]);
                old_cursor += 1;
                new_cursor += 1;
            }
            Edit::Delete { old: old_index } => {
                assert_eq!(old_index, old_cursor);
                old_cursor += 1;
            }
            Edit::Insert { new: new_index } => {
                assert_eq!(new_index, new_cursor);
                new_cursor += 1;
            }
        }
    }
    assert_eq!(old_cursor, old.len());
    assert_eq!(new_cursor, new.len());
}

fn reference_edit_distance(old: &[u8], new: &[u8]) -> usize {
    let mut previous = (0..=new.len()).collect::<Vec<_>>();
    for (old_index, old_value) in old.iter().enumerate() {
        let mut current = vec![old_index + 1; new.len() + 1];
        for (new_index, new_value) in new.iter().enumerate() {
            current[new_index + 1] = if old_value == new_value {
                previous[new_index]
            } else {
                1 + previous[new_index + 1].min(current[new_index])
            };
        }
        previous = current;
    }
    previous[new.len()]
}
