use super::HttpRuntime;
use super::serve_listener;
use crate::CollaborationServerOptions;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use tempfile::TempDir;
use zeta_collaboration::SqliteDocumentCollaborationRooms;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";
const ORIGIN: &str = "https://desktop.zeta.example";

#[test]
fn remote_host_authenticates_origins_and_orders_cross_client_updates() {
    let directory = TempDir::new().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let options =
        CollaborationServerOptions::new(address, directory.path().join("rooms.sqlite3"), TOKEN)
            .with_allowed_origin(ORIGIN);
    let runtime = Arc::new(HttpRuntime::new(
        SqliteDocumentCollaborationRooms::open_at(options.database_path()).unwrap(),
        options,
    ));
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_runtime = runtime.clone();
    let worker_shutdown = shutdown.clone();
    let worker =
        thread::spawn(move || serve_listener(listener, worker_runtime, worker_shutdown).unwrap());

    let unauthorized = send(
        address,
        "POST",
        "/v1/document-collaboration/rooms/open",
        &[("Origin", ORIGIN), ("Content-Type", "application/json")],
        b"{}",
    );
    assert_eq!(unauthorized.status, 401);
    assert_eq!(
        unauthorized.headers.get("access-control-allow-origin"),
        Some(&ORIGIN.into())
    );
    let rejected_origin = send(
        address,
        "OPTIONS",
        "/v1/document-collaboration/rooms/open",
        &[("Origin", "https://other.example")],
        b"",
    );
    assert_eq!(rejected_origin.status, 403);
    let preflight = send(
        address,
        "OPTIONS",
        "/v1/document-collaboration/rooms/open",
        &[("Origin", ORIGIN)],
        b"",
    );
    assert_eq!(preflight.status, 204);
    assert_eq!(
        preflight.headers.get("access-control-allow-origin"),
        Some(&ORIGIN.into())
    );

    let opened = send_json(
        address,
        "/v1/document-collaboration/rooms/open",
        &serde_json::json!({
            "clientId": "client-a",
            "schemaId": "gama-v1",
            "document": document("initial"),
        }),
    );
    assert_eq!(opened.status, 200);
    let opened_body: Value = serde_json::from_slice(&opened.body).unwrap();
    let room_id = opened_body["snapshot"]["roomId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(opened_body["snapshot"]["version"], 0);

    let joined = send_json(
        address,
        "/v1/document-collaboration/rooms/open",
        &serde_json::json!({
            "roomId": room_id,
            "clientId": "client-b",
            "schemaId": "gama-v1",
            "document": document("ignored"),
        }),
    );
    assert_eq!(joined.status, 200);

    let submitted = send_json(
        address,
        "/v1/document-collaboration/rooms/submit",
        &serde_json::json!({
            "roomId": room_id,
            "clientId": "client-a",
            "sequence": 1,
            "baseVersion": 0,
            "transaction": serde_json::json!({"steps": []}).to_string(),
            "document": document("first"),
        }),
    );
    assert_eq!(submitted.status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&submitted.body).unwrap()["status"],
        "accepted"
    );

    let updates = send(
        address,
        "GET",
        &format!("/v1/document-collaboration/rooms/{room_id}/updates?afterVersion=0"),
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Origin", ORIGIN),
        ],
        b"",
    );
    assert_eq!(updates.status, 200);
    let updates_body: Value = serde_json::from_slice(&updates.body).unwrap();
    assert_eq!(updates_body["status"], "updates");
    assert_eq!(updates_body["updates"][0]["clientId"], "client-a");
    assert_eq!(updates_body["updates"][0]["version"], 1);

    shutdown.store(true, Ordering::Release);
    worker.join().unwrap();
}

fn send_json(address: SocketAddr, path: &str, body: &Value) -> HttpResponse {
    let encoded = serde_json::to_vec(body).unwrap();
    send(
        address,
        "POST",
        path,
        &[
            ("Authorization", &format!("Bearer {TOKEN}")),
            ("Origin", ORIGIN),
            ("Content-Type", "application/json"),
        ],
        &encoded,
    )
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
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
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

fn document(value: &str) -> String {
    format!(
        r#"{{"format":"zeta.document","version":1,"document":{{"type":"document","content":["{value}"]}}}}"#
    )
}
