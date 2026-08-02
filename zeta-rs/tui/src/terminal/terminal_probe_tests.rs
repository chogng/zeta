use super::parse_response;
use zeta_terminal_detection::TerminalRgb;

#[test]
fn parses_xterm_rgb_response_with_bel_terminator() {
    assert_eq!(
        parse_response(b"\x1b]11;rgb:ffff/8000/0000\x07"),
        Some(TerminalRgb::new(255, 128, 0))
    );
}

#[test]
fn parses_rgb_response_with_string_terminator_and_variable_precision() {
    assert_eq!(
        parse_response(b"prefix\x1b]11;rgb:f/8/0\x1b\\suffix"),
        Some(TerminalRgb::new(255, 136, 0))
    );
}

#[test]
fn parses_c1_osc_and_hash_rgb_responses() {
    assert_eq!(
        parse_response(b"\x9d11;#1f2328\x07"),
        Some(TerminalRgb::new(31, 35, 40))
    );
}

#[test]
fn rejects_incomplete_or_malformed_responses() {
    assert_eq!(parse_response(b"\x1b]11;rgb:ffff/ffff/ffff"), None);
    assert_eq!(parse_response(b"\x1b]11;rgb:zz/00/00\x07"), None);
    assert_eq!(parse_response(b"\x1b]10;rgb:ffff/ffff/ffff\x07"), None);
}
