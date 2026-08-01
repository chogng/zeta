use super::*;

#[test]
fn terminal_environment_keeps_safe_values_and_excludes_secrets() {
    let environment = TerminalEnvironment::from_variables([
        ("HOME".into(), "/home/zeta".into()),
        ("LANG".into(), "en_US.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
        ("PATH".into(), "/usr/bin".into()),
        ("OPENAI_API_KEY".into(), "secret".into()),
        ("AWS_SECRET_ACCESS_KEY".into(), "secret".into()),
    ]);

    assert_eq!(
        environment.variables().get("HOME").map(String::as_str),
        Some("/home/zeta")
    );
    assert_eq!(
        environment.variables().get("LC_ALL").map(String::as_str),
        Some("C.UTF-8")
    );
    assert!(!environment.variables().contains_key("OPENAI_API_KEY"));
    assert!(
        !environment
            .variables()
            .contains_key("AWS_SECRET_ACCESS_KEY")
    );
}

#[test]
fn terminal_environment_owns_terminal_identity_values() {
    let environment = TerminalEnvironment::from_variables([
        ("TERM".into(), "host-term".into()),
        ("COLORTERM".into(), "host-color".into()),
        ("TERM_PROGRAM".into(), "host-program".into()),
    ]);

    assert_eq!(environment.variables()["TERM"], "xterm-256color");
    assert_eq!(environment.variables()["COLORTERM"], "truecolor");
    assert_eq!(environment.variables()["TERM_PROGRAM"], "zeta");
}

#[test]
fn terminal_environment_rejects_invalid_names_and_values() {
    let environment = TerminalEnvironment::from_variables([
        ("BAD=NAME".into(), "value".into()),
        ("HOME".into(), "bad\0value".into()),
        ("\0PATH".into(), "/bin".into()),
    ]);

    assert!(!environment.variables().contains_key("BAD=NAME"));
    assert!(!environment.variables().contains_key("HOME"));
    assert!(!environment.variables().contains_key("\0PATH"));
}

#[cfg(not(windows))]
#[test]
fn posix_terminal_environment_keeps_variable_names_case_sensitive() {
    let environment = TerminalEnvironment::from_variables([
        ("PATH".into(), "/usr/bin".into()),
        ("Path".into(), "/untrusted".into()),
    ]);

    assert_eq!(environment.variables()["PATH"], "/usr/bin");
    assert!(!environment.variables().contains_key("Path"));
}

#[cfg(windows)]
#[test]
fn windows_terminal_environment_canonicalizes_variable_names() {
    let environment = TerminalEnvironment::from_variables([
        ("Path".into(), r"C:\Windows\System32".into()),
        ("SystemRoot".into(), r"C:\Windows".into()),
    ]);

    assert_eq!(environment.variables()["PATH"], r"C:\Windows\System32");
    assert_eq!(environment.variables()["SYSTEMROOT"], r"C:\Windows");
}
