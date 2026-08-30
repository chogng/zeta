#![cfg(unix)]

use std::fs;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use portable_pty::CommandBuilder;
use portable_pty::ExitStatus;
use portable_pty::PtySize;
use portable_pty::native_pty_system;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const STATE_TIMEOUT: Duration = Duration::from_secs(30);
const OUTPUT_LIMIT: usize = 256 * 1024;

#[test]
fn interactive_remote_tui_recovers_the_durable_session_after_transport_loss() {
    let root = test_root("interactive-reconnect");
    let dir = root.join("dir");
    let profile_root = root.join("profile");
    let fake_ssh = root.join("fake-ssh");
    let connection_count = root.join("connection-count");
    let recovered_requests = root.join("recovered-requests.jsonl");
    fs::create_dir_all(&dir).unwrap();
    write_reconnecting_fake_ssh(
        &fake_ssh,
        &profile_root,
        &connection_count,
        &recovered_requests,
    );

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 32,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let output = Arc::new(Mutex::new(Vec::new()));
    let reader_output = Arc::clone(&output);
    let reader_thread = thread::spawn(move || {
        let mut buffer = [0_u8; 8_192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let mut output = reader_output.lock().unwrap();
                    let remaining = OUTPUT_LIMIT.saturating_sub(output.len());
                    output.extend_from_slice(&buffer[..read.min(remaining)]);
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });

    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_zeta"));
    command.args([
        "remote",
        "connect",
        "--host",
        "local-reconnect-double",
        "--dir",
        dir.to_str().unwrap(),
        "--runtime",
        env!("CARGO_BIN_EXE_zeta"),
        "--ssh",
        fake_ssh.to_str().unwrap(),
    ]);
    command.cwd(&dir);
    command.env("TERM", "xterm-256color");
    command.env("ZETA_PROFILE_ROOT", &profile_root);
    command.env("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "5000");
    let mut child = ChildGuard::new(pair.slave.spawn_command(command).unwrap());
    drop(pair.slave);

    assert!(
        wait_for_file(&recovered_requests, STATE_TIMEOUT, |contents| {
            contents.contains("\"method\":\"session/read\"")
                && contents.contains("\"method\":\"session/thread/read\"")
        }),
        "second TUI generation did not read the durable Session and Thread; output:\n{}",
        captured_output(&output)
    );
    assert!(
        wait_for_output_occurrences(&output, b"Tips for getting started", 2, STATE_TIMEOUT,),
        "second TUI generation did not draw its ready frame; output:\n{}",
        captured_output(&output)
    );
    writer.write_all(&[0x03]).unwrap();
    writer.flush().unwrap();

    let deadline = Instant::now() + PROCESS_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.terminate();
            panic!(
                "interactive Remote TUI did not exit after Ctrl-C; output:\n{}",
                captured_output(&output)
            );
        }
        thread::sleep(Duration::from_millis(25));
    };

    drop(writer);
    drop(pair.master);
    reader_thread.join().unwrap();
    assert!(status.success(), "output:\n{}", captured_output(&output));
    assert_eq!(
        fs::read_to_string(&connection_count)
            .unwrap()
            .trim()
            .parse::<usize>()
            .unwrap(),
        2,
        "the host should establish exactly one replacement connection"
    );
    let output = captured_output(&output);
    assert!(output.contains("Remote App Server disconnected:"));
    assert!(output.contains("Reconnecting to Remote App Server"));

    thread::sleep(Duration::from_millis(5_500));
    fs::remove_dir_all(root).unwrap();
}

struct ChildGuard {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    running: bool,
}

impl ChildGuard {
    fn new(child: Box<dyn portable_pty::Child + Send + Sync>) -> Self {
        Self {
            child,
            running: true,
        }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.running = false;
        }
        Ok(status)
    }

    fn terminate(&mut self) {
        if self.running {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.running = false;
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn write_reconnecting_fake_ssh(
    path: &Path,
    profile_root: &Path,
    connection_count: &Path,
    recovered_requests: &Path,
) {
    let first_requests = connection_count.with_file_name("first-requests.jsonl");
    let request_fifo = connection_count.with_file_name("first-requests.fifo");
    let response_fifo = connection_count.with_file_name("first-responses.fifo");
    for value in [
        path,
        profile_root,
        connection_count,
        recovered_requests,
        &first_requests,
        &request_fifo,
        &response_fifo,
        Path::new(env!("CARGO_BIN_EXE_zeta")),
    ] {
        assert!(!value.to_string_lossy().contains('\''));
    }
    fs::write(
        path,
        format!(
            "#!/bin/sh\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\ncase \"$command\" in\n  *__ZETA_REMOTE_RUNTIME_FOUND__*) printf '%s\\n' '__ZETA_REMOTE_RUNTIME_FOUND__:{runtime}' ;;
  *\"'remote-server' 'connect'\"*)\n    export ZETA_PROFILE_ROOT='{profile}'\n    count=0\n    if [ -f '{count}' ]; then count=$(cat '{count}'); fi\n    count=$((count + 1))\n    printf '%s\\n' \"$count\" > '{count}'\n    if [ \"$count\" -eq 1 ]; then\n      rm -f '{request_fifo}' '{response_fifo}' '{first_requests}'\n      mkfifo '{request_fifo}' '{response_fifo}'\n      exec 3<&0\n      /bin/sh -c \"$command\" < '{request_fifo}' > '{response_fifo}' &\n      server=$!\n      tee -a '{first_requests}' <&3 > '{request_fifo}' &\n      request_relay=$!\n      cat '{response_fifo}' &\n      response_relay=$!\n      attempt=0\n      while [ \"$attempt\" -lt 30 ]; do\n        if grep -q '\"method\":\"git/status\"' '{first_requests}' 2>/dev/null; then break; fi\n        sleep 1\n        attempt=$((attempt + 1))\n      done\n      sleep 1\n      kill \"$server\" \"$request_relay\" \"$response_relay\" 2>/dev/null || true\n      wait \"$server\" 2>/dev/null\n      rm -f '{request_fifo}' '{response_fifo}'\n      exit 255\n    fi\n    tee -a '{requests}' | /bin/sh -c \"$command\"\n    ;;
  *) exit 64 ;;
esac\n",
            runtime = env!("CARGO_BIN_EXE_zeta"),
            profile = profile_root.display(),
            count = connection_count.display(),
            requests = recovered_requests.display(),
            first_requests = first_requests.display(),
            request_fifo = request_fifo.display(),
            response_fifo = response_fifo.display(),
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn wait_for_file(path: &Path, timeout: Duration, predicate: impl Fn(&str) -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(contents) = fs::read_to_string(path)
            && predicate(&contents)
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn captured_output(output: &Arc<Mutex<Vec<u8>>>) -> String {
    String::from_utf8_lossy(&output.lock().unwrap()).into_owned()
}

fn wait_for_output_occurrences(
    output: &Arc<Mutex<Vec<u8>>>,
    pattern: &[u8],
    expected: usize,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let occurrences = output
            .lock()
            .unwrap()
            .windows(pattern.len())
            .filter(|window| *window == pattern)
            .count();
        if occurrences >= expected {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-cli-remote-connect-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ))
}
