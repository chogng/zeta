use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn binary_serves_initialize_and_tool_catalog_over_stdio() {
    let state_root = std::env::temp_dir().join(format!(
        "zeta-mcp-server-stdio-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut child = Command::new(env!("CARGO_BIN_EXE_zeta-mcp-server"))
        .env("ZETA_PROFILE_ROOT", &state_root)
        .env("ZETA_WORKSPACE_ROOT", std::env::current_dir().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("MCP server binary starts");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-11-25","clientInfo":{{"name":"process-test","version":"1"}},"capabilities":{{}}}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let initialized = read_response(&mut stdout);
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "zeta-mcp-server"
    );

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let tools = read_response(&mut stdout);
    assert_eq!(tools["result"]["tools"][0]["name"], "zeta");
    assert_eq!(tools["result"]["tools"][1]["name"], "zeta-reply");

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"_meta":{{"progressToken":"stdio-progress"}},"name":"zeta","arguments":{{"invocationId":"stdio-call-1","prompt":"inspect"}}}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    let mut saw_progress = false;
    let final_result = loop {
        let message = read_response(&mut stdout);
        if message["method"] == "notifications/progress" {
            saw_progress = true;
            assert_eq!(
                message["params"]["progressToken"],
                Value::String("stdio-progress".into())
            );
        }
        if message["id"] == 3 {
            break message;
        }
    };
    assert!(saw_progress);
    assert_eq!(final_result["result"]["isError"], true);
    let first_structured = final_result["result"]["structuredContent"].clone();
    let thread_id = first_structured["threadId"].as_str().unwrap().to_string();

    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "server failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut restarted = Command::new(env!("CARGO_BIN_EXE_zeta-mcp-server"))
        .env("ZETA_PROFILE_ROOT", &state_root)
        .env("ZETA_WORKSPACE_ROOT", std::env::current_dir().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("restarted MCP server binary starts");
    let mut restarted_stdin = restarted.stdin.take().unwrap();
    let mut restarted_stdout = BufReader::new(restarted.stdout.take().unwrap());
    writeln!(
        restarted_stdin,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-11-25","clientInfo":{{"name":"restart-test","version":"1"}},"capabilities":{{}}}}}}"#
    )
    .unwrap();
    restarted_stdin.flush().unwrap();
    read_response(&mut restarted_stdout);

    writeln!(
        restarted_stdin,
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"zeta","arguments":{{"invocationId":"stdio-call-1","prompt":"inspect"}}}}}}"#
    )
    .unwrap();
    restarted_stdin.flush().unwrap();
    let replay = read_response(&mut restarted_stdout);
    assert_eq!(replay["result"]["structuredContent"], first_structured);

    writeln!(
        restarted_stdin,
        r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"zeta-reply","arguments":{{"invocationId":"stdio-reply-after-restart","threadId":"{thread_id}","prompt":"continue"}}}}}}"#
    )
    .unwrap();
    restarted_stdin.flush().unwrap();
    let continued = read_response(&mut restarted_stdout);
    assert_eq!(
        continued["result"]["structuredContent"]["threadId"],
        thread_id
    );

    drop(restarted_stdin);
    let restarted_output = restarted.wait_with_output().unwrap();
    assert!(
        restarted_output.status.success(),
        "restarted server failed: {}",
        String::from_utf8_lossy(&restarted_output.stderr)
    );
    fs::remove_dir_all(&state_root).unwrap();
}

fn read_response(reader: &mut impl BufRead) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(!line.is_empty(), "server closed stdout without a response");
    serde_json::from_str(&line).unwrap()
}
