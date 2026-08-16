use super::*;

#[test]
fn mcp_http_listener_requires_an_ip_port_and_endpoint_path() {
    let (address, path) = parse_mcp_http_address("http://127.0.0.1:8787/mcp").unwrap();
    assert_eq!(address, "127.0.0.1:8787".parse().unwrap());
    assert_eq!(path, "/mcp");
    assert!(parse_mcp_http_address("https://127.0.0.1:8787/mcp").is_err());
    assert!(parse_mcp_http_address("http://localhost:8787/mcp").is_err());
    assert!(parse_mcp_http_address("http://127.0.0.1:8787").is_err());
}

#[test]
fn exec_arguments_default_to_a_safe_new_human_run() {
    let options = parse_exec_arguments(vec!["inspect".into(), "the workspace".into()]).unwrap();
    assert_eq!(options.entry, HeadlessEntry::New);
    assert_eq!(options.prompt, "inspect the workspace");
    assert_eq!(options.output, ExecOutputMode::Human);
    assert_eq!(
        options.approval,
        HeadlessApprovalMode::DenyInteractiveRequests
    );
}

#[test]
fn exec_arguments_parse_jsonl_resume_and_auto_review() {
    let options = parse_exec_arguments(vec![
        "--jsonl".into(),
        "--auto-review".into(),
        "--resume".into(),
        "session-1".into(),
        "thread-1".into(),
        "continue".into(),
    ])
    .unwrap();
    assert_eq!(
        options.entry,
        HeadlessEntry::Resume {
            session_id: SessionId::new("session-1").unwrap(),
            thread_id: ThreadId::new("thread-1").unwrap(),
        }
    );
    assert_eq!(options.output, ExecOutputMode::JsonLines);
    assert_eq!(options.approval, HeadlessApprovalMode::AutomaticReview);
}

#[test]
fn exec_arguments_reject_conflicting_authority_and_entry_modes() {
    assert!(
        parse_exec_arguments(vec![
            "--auto-review".into(),
            "--dangerously-bypass-permissions".into(),
            "task".into(),
        ])
        .is_err()
    );
    assert!(
        parse_exec_arguments(vec![
            "--resume".into(),
            "session-1".into(),
            "thread-1".into(),
            "--fork".into(),
            "session-1".into(),
            "thread-2".into(),
            "task".into(),
        ])
        .is_err()
    );
}
