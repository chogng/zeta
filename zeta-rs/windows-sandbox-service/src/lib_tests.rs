use super::RunMode;
use super::parse_mode;

#[test]
fn defaults_to_service_mode() {
    assert_eq!(parse_mode(std::iter::empty()).unwrap(), RunMode::Service);
}

#[test]
fn accepts_explicit_service_mode() {
    assert_eq!(
        parse_mode([String::from("--service")].into_iter()).unwrap(),
        RunMode::Service
    );
}

#[test]
#[cfg(debug_assertions)]
fn accepts_foreground_mode_for_development() {
    assert_eq!(
        parse_mode([String::from("--foreground")].into_iter()).unwrap(),
        RunMode::Foreground
    );
}

#[test]
fn rejects_unknown_or_multiple_arguments() {
    assert!(parse_mode([String::from("--unknown")].into_iter()).is_err());
    assert!(
        parse_mode([String::from("--service"), String::from("--foreground")].into_iter()).is_err()
    );
}
