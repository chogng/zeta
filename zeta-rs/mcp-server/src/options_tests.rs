use super::*;

#[test]
fn defaults_bound_one_turn() {
    let options = McpServerOptions::new("state", "workspace");
    let limits = options.runtime_limits();

    assert_eq!(limits.default_turn_timeout, Duration::from_secs(60));
    assert_eq!(limits.maximum_turn_timeout, Duration::from_secs(600));
    assert_eq!(limits.poll_interval, Duration::from_millis(10));
    assert_eq!(options.validate(), Ok(()));
}

#[test]
fn invalid_runtime_limits_are_rejected() {
    let options = McpServerOptions::new("state", "workspace")
        .with_default_turn_timeout(Duration::from_secs(2))
        .with_maximum_turn_timeout(Duration::from_secs(1));

    assert!(options.validate().is_err());
}

#[test]
fn http_options_require_a_strong_token_and_safe_endpoint() {
    let address = "127.0.0.1:8787".parse().unwrap();
    assert!(
        HttpServerOptions::new(address, "/mcp", "0123456789abcdef0123456789abcdef")
            .validate()
            .is_ok()
    );
    assert!(
        HttpServerOptions::new(address, "mcp", "0123456789abcdef0123456789abcdef")
            .validate()
            .is_err()
    );
    let weak = HttpServerOptions::new(address, "/mcp", "weak");
    assert!(weak.validate().is_err());
    assert!(!format!("{weak:?}").contains("weak"));
}
