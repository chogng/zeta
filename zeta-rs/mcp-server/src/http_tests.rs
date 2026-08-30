use super::wire::find_header_end;
use super::*;
use crate::options::McpServerOptions;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_client::{InProcessClientOptions, open_in_process_app_server};
use zeta_app_server_protocol::protocol::common::ClientInfo;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

#[test]
fn streamable_http_enforces_auth_session_origin_and_protocol() {
    let state_root = temporary_state_root();
    let mcp_options = McpServerOptions::new(&state_root, std::env::current_dir().unwrap());
    let host = open_in_process_app_server(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "http-test".into(),
                version: "1".into(),
            },
        )
        .with_dir_root(std::env::current_dir().unwrap()),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let options = HttpServerOptions::new(address, "/mcp", TOKEN)
        .with_allowed_origin("https://client.example");
    let runtime = Arc::new(HttpRuntime::new(
        host,
        mcp_options.runtime_limits(),
        Arc::new(ReceiptStore::memory()),
        options,
    ));
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_runtime = runtime.clone();
    let worker_shutdown = shutdown.clone();
    let worker =
        thread::spawn(move || serve_listener(listener, worker_runtime, worker_shutdown).unwrap());

    let unauthorized = send(address, "POST", "/mcp", &[], b"{}");
    assert_eq!(unauthorized.status, 401);
    let duplicate_authorization = send(
        address,
        "POST",
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Authorization", &format!("Bearer {TOKEN}")),
        ],
        b"{}",
    );
    assert_eq!(duplicate_authorization.status, 400);
    let transfer_encoded = send(
        address,
        "POST",
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Transfer-Encoding", "chunked"),
        ],
        b"{}",
    );
    assert_eq!(transfer_encoded.status, 400);

    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}
        }
    });
    let initialized = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Origin", "https://client.example"),
        ],
        &initialize,
    );
    assert_eq!(initialized.status, 200);
    let session_id = initialized.headers.get(SESSION_HEADER).unwrap().clone();
    assert_eq!(
        serde_json::from_slice::<Value>(&initialized.body).unwrap()["result"]["protocolVersion"],
        MCP_PROTOCOL_VERSION
    );

    let rejected_origin = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Origin", "https://evil.example"),
        ],
        &initialize,
    );
    assert_eq!(rejected_origin.status, 403);

    let tools = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}});
    let listed = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("MCP-Session-Id", &session_id),
            ("MCP-Protocol-Version", MCP_PROTOCOL_VERSION),
        ],
        &tools,
    );
    assert_eq!(listed.status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&listed.body).unwrap()["result"]["tools"][0]["name"],
        "zeta"
    );

    let call = json!({
        "jsonrpc":"2.0",
        "id":3,
        "method":"tools/call",
        "params":{
            "_meta":{"progressToken":"http-progress"},
            "name":"zeta",
            "arguments":{"invocationId":"http-call-1","prompt":"inspect"}
        }
    });
    let streamed = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("MCP-Session-Id", &session_id),
            ("MCP-Protocol-Version", MCP_PROTOCOL_VERSION),
        ],
        &call,
    );
    assert_eq!(streamed.status, 200);
    assert_eq!(
        streamed.headers.get("content-type").map(String::as_str),
        Some("text/event-stream")
    );
    let stream_body = String::from_utf8(streamed.body).unwrap();
    assert!(stream_body.contains("notifications/progress"));
    assert!(stream_body.contains(r#""id":3"#));

    let missing_protocol = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("MCP-Session-Id", &session_id),
        ],
        &tools,
    );
    assert_eq!(missing_protocol.status, 400);

    let deleted = send(
        address,
        "DELETE",
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("MCP-Session-Id", &session_id),
            ("MCP-Protocol-Version", MCP_PROTOCOL_VERSION),
        ],
        b"",
    );
    assert_eq!(deleted.status, 204);

    let expired = send_json(
        address,
        "/mcp",
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("MCP-Session-Id", &session_id),
            ("MCP-Protocol-Version", MCP_PROTOCOL_VERSION),
        ],
        &tools,
    );
    assert_eq!(expired.status, 404);

    shutdown.store(true, Ordering::Release);
    worker.join().unwrap();
    fs::remove_dir_all(state_root).unwrap();
}

fn send_json(
    address: SocketAddr,
    path: &str,
    headers: &[(&str, &str)],
    body: &Value,
) -> HttpResponse {
    let encoded = serde_json::to_vec(body).unwrap();
    let mut all_headers = vec![
        ("Accept", "application/json, text/event-stream"),
        ("Content-Type", "application/json"),
    ];
    all_headers.extend_from_slice(headers);
    send(address, "POST", path, &all_headers, &encoded)
}

fn send(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> HttpResponse {
    let mut stream = TcpStream::connect(address).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
        body.len()
    )
    .unwrap();
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n").unwrap();
    }
    write!(stream, "\r\n").unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    decode_response(&response)
}

struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn decode_response(bytes: &[u8]) -> HttpResponse {
    let header_end = find_header_end(bytes).unwrap();
    let head = std::str::from_utf8(&bytes[..header_end]).unwrap();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().into()))
        .collect();
    HttpResponse {
        status,
        headers,
        body: bytes[header_end + 4..].to_vec(),
    }
}

fn temporary_state_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-mcp-http-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
