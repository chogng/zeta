use super::background;
use super::response_ranges;
use zeta_terminal_detection::TerminalRgb;

#[test]
fn finds_a_valid_background_reply_among_unrelated_input() {
    let input = b"a\x1b]10;rgb:eeee/eeee/eeee\x1b\\b\x1b]11;rgb:f5f5/f5f5/f5f5\x07c";

    assert_eq!(background(input), Some(TerminalRgb::new(245, 245, 245)));
    assert_eq!(response_ranges(input), vec![27..51]);
}

#[test]
fn leaves_osc_looking_bracketed_paste_content_alone() {
    let input = b"\x1b[200~\x1b]11;rgb:ffff/ffff/ffff\x07\x1b[201~x";

    assert_eq!(background(input), None);
    assert!(response_ranges(input).is_empty());
}

#[test]
fn ignores_incomplete_and_malformed_replies() {
    let input = b"\x1b]11;rgb:ffff/ffff/ffff\x1b]11;rgb:nope\x07";

    assert_eq!(background(input), None);
    assert!(response_ranges(input).is_empty());
}
