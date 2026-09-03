//! Identify terminal background replies without changing unrelated Windows console input.

use std::ops::Range;

use zeta_terminal_detection::TerminalRgb;

const MAX_RESPONSE_BYTES: usize = 1_024;
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

pub(super) fn response_ranges(input: &[u8]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0;

    while cursor < input.len() {
        if input[cursor..].starts_with(PASTE_START) {
            let payload_start = cursor + PASTE_START.len();
            let Some(payload_end) = super::find_bytes(&input[payload_start..], PASTE_END) else {
                break;
            };
            cursor = payload_start + payload_end + PASTE_END.len();
            continue;
        }

        let prefix_len = if input[cursor..].starts_with(b"\x1b]11;") {
            b"\x1b]11;".len()
        } else if input[cursor..].starts_with(b"\x9d11;") {
            b"\x9d11;".len()
        } else {
            cursor += 1;
            continue;
        };

        let payload_start = cursor + prefix_len;
        let bounded_end = input.len().min(cursor.saturating_add(MAX_RESPONSE_BYTES));
        let Some((payload_len, terminator_len)) = payload_end(&input[payload_start..bounded_end])
        else {
            cursor = payload_start;
            continue;
        };
        let end = payload_start + payload_len + terminator_len;
        if super::parse_response(&input[cursor..end]).is_some() {
            ranges.push(cursor..end);
        }
        cursor = end;
    }

    ranges
}

pub(super) fn background(input: &[u8]) -> Option<TerminalRgb> {
    response_ranges(input)
        .into_iter()
        .find_map(|range| super::parse_response(&input[range]))
}

fn payload_end(input: &[u8]) -> Option<(usize, usize)> {
    let mut index = 0;
    while index < input.len() {
        match input[index] {
            0x07 => return Some((index, 1)),
            0x1b if input.get(index + 1) == Some(&b'\\') => return Some((index, 2)),
            _ => index += 1,
        }
    }
    None
}

#[cfg(test)]
#[path = "windows_replay_tests.rs"]
mod tests;
