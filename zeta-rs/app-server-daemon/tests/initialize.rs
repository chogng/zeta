#![cfg(any(unix, windows))]

use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::process::Child;
use std::process::Command;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use serde_json::json;
use zeta_app_server_daemon::daemon_endpoint_path;
use zeta_uds::UnixStream;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn daemon_answers_initialize_for_the_repository_workspace() {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("p");
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap()
        .to_path_buf();
    let product_services = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/product-services.json");
    fs::create_dir(&profile).unwrap();
    let endpoint = daemon_endpoint_path(&profile).unwrap();
    let daemon = Command::new(env!("CARGO_BIN_EXE_zeta-app-server-daemon"))
        .env("ZETA_PROFILE_ROOT", &profile)
        .env("ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS", "5000")
        .spawn()
        .unwrap();
    let _daemon = Daemon(daemon);
    let mut stream = connect_when_ready(&endpoint);
    stream.set_read_timeout(Some(CONNECT_TIMEOUT)).unwrap();
    writeln!(
        stream,
        "{}",
        json!({
            "version": 1,
            "workspaceRoot": workspace,
            "workspaceTrustSource": "hostConfiguration",
            "productServices": product_services,
        })
    )
    .unwrap();
    writeln!(
        stream,
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"clientInfo":{{"name":"daemon-test","version":"1"}},"capabilities":{{}}}}}}"#
    )
    .unwrap();
    stream.flush().unwrap();

    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).unwrap();
    let response: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["serverInfo"]["name"], "zeta-app-server");
    assert!(response["result"]["schemaHash"].as_str().is_some());
}

fn connect_when_ready(endpoint: &std::path::Path) -> UnixStream {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        match UnixStream::connect(endpoint) {
            Ok(stream) => return stream,
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => panic!("daemon endpoint did not become ready: {error}"),
        }
    }
}
