use super::*;

#[test]
fn tool_schema_requires_idempotency_identity() {
    let tools = tools_result();
    let definitions = tools["tools"].as_array().unwrap();

    assert_eq!(definitions[0]["name"], TOOL_START);
    assert_eq!(
        definitions[0]["inputSchema"]["required"],
        json!(["invocationId", "prompt"])
    );
    assert_eq!(definitions[1]["name"], TOOL_REPLY);
}

#[test]
fn progress_token_accepts_only_mcp_string_or_integer_identity() {
    let valid: CallToolParams = serde_json::from_value(json!({
        "name": "zeta",
        "arguments": {},
        "_meta": {"progressToken": "progress-1"}
    }))
    .unwrap();
    assert_eq!(valid.progress_token().unwrap(), Some(json!("progress-1")));

    let invalid: CallToolParams = serde_json::from_value(json!({
        "name": "zeta",
        "arguments": {},
        "_meta": {"progressToken": {"nested": true}}
    }))
    .unwrap();
    assert!(invalid.progress_token().is_err());
}

#[test]
fn start_rejects_unsafe_or_oversized_identity() {
    let unsafe_id = json!({"invocationId": "bad:id", "prompt": "hello"});
    let oversized_id = json!({
        "invocationId": "a".repeat(MAX_INVOCATION_ID_BYTES + 1),
        "prompt": "hello"
    });

    assert!(decode_start(unsafe_id).is_err());
    assert!(decode_start(oversized_id).is_err());
}

#[test]
fn reply_requires_nonempty_thread_and_prompt() {
    let arguments = json!({
        "invocationId": "reply-1",
        "threadId": "",
        "prompt": " "
    });

    assert!(decode_reply(arguments).is_err());
}
