use portable_pty::CommandBuilder;
use portable_pty::ExitStatus;
use portable_pty::MasterPty;
use portable_pty::PtySize;
use portable_pty::native_pty_system;
use std::fs;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_terminal::GridSize;
use zeta_terminal::TerminalCore;

const PROCESS_TIMEOUT: Duration = Duration::from_secs(20);
const STATE_TIMEOUT: Duration = Duration::from_secs(30);
const REDRAW_QUIET_PERIOD: Duration = Duration::from_millis(40);
const OUTPUT_LIMIT: usize = 512 * 1024;
pub const LARGE_SIZE: PtySize = PtySize {
    rows: 32,
    cols: 100,
    pixel_width: 0,
    pixel_height: 0,
};
pub const SMALL_SIZE: PtySize = PtySize {
    rows: 16,
    cols: 60,
    pixel_width: 0,
    pixel_height: 0,
};

pub struct Fixture {
    _root: TempDir,
    workspace: PathBuf,
    profile: PathBuf,
    fake_ssh: PathBuf,
}

impl Fixture {
    pub fn new(name: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix(&format!("zeta-tui-{name}-"))
            .tempdir()
            .unwrap();
        let workspace = root.path().join("workspace");
        let profile = root.path().join("profile");
        let fake_ssh = root.path().join("fake-ssh");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&profile).unwrap();
        let runtime = env!("CARGO_BIN_EXE_zeta");
        assert!(!profile.to_string_lossy().contains('\''));
        fs::write(
            &fake_ssh,
            format!(
                "#!/bin/sh\ncommand=''\nfor argument in \"$@\"; do command=$argument; done\nexport ZETA_PROFILE_ROOT='{}'\nexec /bin/sh -c \"$command\"\n",
                profile.display()
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_ssh).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_ssh, permissions).unwrap();
        assert!(Path::new(runtime).is_absolute());
        Self {
            _root: root,
            workspace,
            profile,
            fake_ssh,
        }
    }

    pub fn write_config(&self, base_url: &str) {
        fs::write(
            self.profile.join("config.toml"),
            format!(
                r#"[agent.preferredModel]
provider = "openai-compatible"
model = "zeta-real-scenario"

[providers."openai-compatible"]
provider = "openai-compatible"
baseUrl = "{base_url}"
"#,
            ),
        )
        .unwrap();
    }

    pub fn workspace(&self) -> &Path {
        &self.workspace
    }

    pub fn find_file(&self, name: &str) -> Option<PathBuf> {
        find_named(self._root.path(), name)
    }

    pub fn only_thread(&self) -> (String, String) {
        let command = StdioAppServerCommand::new(env!("CARGO_BIN_EXE_zeta"))
            .with_argument("remote-server")
            .with_argument("connect")
            .with_environment_variable("ZETA_PROFILE_ROOT", &self.profile)
            .with_environment_variable("ZETA_WORKSPACE_ROOT", &self.workspace);
        let session = AppServerSession::start_stdio(
            command,
            ClientInfo {
                name: "zeta-tui-real-scenario-inspector".into(),
                version: "1".into(),
            },
            zeta_tui::client_capabilities(),
        )
        .unwrap();
        let sessions = session.client().list_sessions().unwrap().sessions;
        session.shutdown().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].threads.len(), 1);
        (
            sessions[0].session_id.to_string(),
            sessions[0].threads[0].thread_id.to_string(),
        )
    }
}

fn find_named(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_named(&path, name) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|file_name| file_name == name) {
            return Some(path);
        }
    }
    None
}

pub struct TuiProcess {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: ChildGuard,
    capture: Arc<Mutex<TerminalCapture>>,
    reader: Option<thread::JoinHandle<()>>,
    snapshot_paths: Vec<String>,
}

impl TuiProcess {
    pub fn start(fixture: &Fixture, args: &[&str], size: PtySize) -> Self {
        let pair = native_pty_system().openpty(size).unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let capture = Arc::new(Mutex::new(TerminalCapture::new(size)));
        let reader_capture = Arc::clone(&capture);
        let reader_thread = thread::spawn(move || {
            let mut buffer = [0_u8; 8_192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => reader_capture.lock().unwrap().push(&buffer[..read]),
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
            "local-real-scenario",
            "--dir",
            fixture.workspace.to_str().unwrap(),
            "--runtime",
            env!("CARGO_BIN_EXE_zeta"),
            "--ssh",
            fixture.fake_ssh.to_str().unwrap(),
        ]);
        command.args(args);
        command.cwd(&fixture.workspace);
        command.env("TERM", "xterm-256color");
        command.env("ZETA_PROFILE_ROOT", &fixture.profile);
        command.env("ZETA_WORKSPACE_ROOT", &fixture.workspace);
        command.env("ZETA_REMOTE_SERVER_IDLE_TIMEOUT_MILLIS", "1000");
        let child = ChildGuard::new(pair.slave.spawn_command(command).unwrap());
        drop(pair.slave);
        let mut snapshot_paths = vec![fixture._root.path().to_string_lossy().into_owned()];
        if let Ok(path) = fs::canonicalize(fixture._root.path()) {
            let path = path.to_string_lossy().into_owned();
            if !snapshot_paths.contains(&path) {
                snapshot_paths.push(path);
            }
        }
        for path in snapshot_paths.clone() {
            if path.starts_with("/var/") {
                snapshot_paths.push(format!("/private{path}"));
            }
        }
        Self {
            master: pair.master,
            writer,
            child,
            capture,
            reader: Some(reader_thread),
            snapshot_paths,
        }
    }

    pub fn submit(&mut self, text: &str) {
        self.type_text(text);
        self.enter();
    }

    pub fn type_text(&mut self, text: &str) {
        self.send_input(text.as_bytes());
    }

    pub fn enter(&mut self) {
        self.send_input(b"\r");
    }

    pub fn tab(&mut self) {
        self.send_input(b"\t");
    }

    pub fn back_tab(&mut self) {
        self.send_input(b"\x1b[Z");
    }

    pub fn up(&mut self) {
        self.send_input(b"\x1b[A");
    }

    pub fn down(&mut self) {
        self.send_input(b"\x1b[B");
    }

    pub fn left(&mut self) {
        self.send_input(b"\x1b[D");
    }

    pub fn right(&mut self) {
        self.send_input(b"\x1b[C");
    }

    pub fn space(&mut self) {
        self.send_input(b" ");
    }

    pub fn escape(&mut self) {
        self.send_input(b"\x1b");
    }

    pub fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).unwrap();
        self.writer.flush().unwrap();
    }

    fn send_input(&mut self, bytes: &[u8]) {
        let revision = self.capture.lock().unwrap().revision();
        self.send(bytes);
        self.wait_for_redraw_after(revision);
    }

    fn wait_for_redraw_after(&mut self, revision: u64) {
        let deadline = Instant::now() + STATE_TIMEOUT;
        let mut observed_revision = None;
        loop {
            let current_revision = self.capture.lock().unwrap().revision();
            if current_revision > revision {
                if observed_revision == Some(current_revision) {
                    return;
                }
                observed_revision = Some(current_revision);
                thread::sleep(REDRAW_QUIET_PERIOD);
                continue;
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!("TUI exited before redrawing after input: {status:?}");
            }
            if Instant::now() >= deadline {
                panic!("TUI did not redraw after input");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn resize(&mut self, size: PtySize) {
        self.master.resize(size).unwrap();
        self.capture.lock().unwrap().resize(size);
    }

    pub fn wait_for_screen(&mut self, expected: &str) {
        let deadline = Instant::now() + STATE_TIMEOUT;
        loop {
            let screen = self.capture.lock().unwrap().screen();
            if screen.contains(expected) {
                return;
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!(
                    "TUI exited before drawing {expected:?}: {status:?}; raw:\n{}",
                    self.capture.lock().unwrap().raw_text()
                );
            }
            if Instant::now() >= deadline {
                panic!(
                    "TUI screen did not contain {expected:?}; screen:\n{screen}\nraw:\n{}",
                    self.capture.lock().unwrap().raw_text()
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn wait_for_output(&mut self, expected: &str) {
        let deadline = Instant::now() + STATE_TIMEOUT;
        loop {
            if self.capture.lock().unwrap().raw_text().contains(expected) {
                return;
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!("TUI exited before emitting {expected:?}: {status:?}");
            }
            if Instant::now() >= deadline {
                panic!("TUI output did not contain {expected:?}");
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn wait_for_stable_screen(&mut self, expected: &str) {
        let deadline = Instant::now() + STATE_TIMEOUT;
        loop {
            let (screen, revision) = {
                let capture = self.capture.lock().unwrap();
                (capture.screen(), capture.revision())
            };
            if screen.contains(expected) {
                thread::sleep(Duration::from_millis(250));
                let capture = self.capture.lock().unwrap();
                if capture.revision() == revision && capture.screen().contains(expected) {
                    return;
                }
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!(
                    "TUI exited before stabilizing {expected:?}: {status:?}; raw:\n{}",
                    self.capture.lock().unwrap().raw_text()
                );
            }
            if Instant::now() >= deadline {
                panic!(
                    "TUI screen did not stabilize with {expected:?}; screen:\n{}",
                    self.capture.lock().unwrap().screen()
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn assert_snapshot(&self, name: &str) {
        let screen =
            normalize_snapshot(self.capture.lock().unwrap().screen(), &self.snapshot_paths);
        assert_named_snapshot(name, screen);
    }

    pub fn assert_snapshot_containing(&self, name: &str, expected: &str) {
        let screen = self
            .capture
            .lock()
            .unwrap()
            .screen_containing(expected)
            .unwrap_or_else(|| panic!("captured output never rendered {expected:?}"));
        assert_named_snapshot(name, normalize_snapshot(screen, &self.snapshot_paths));
    }

    pub fn quit(&mut self) {
        let deadline = Instant::now() + PROCESS_TIMEOUT;
        let mut next_interrupt = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "TUI exited unsuccessfully; output:\n{}",
                    self.capture.lock().unwrap().raw_text()
                );
                break;
            }
            if Instant::now() >= next_interrupt {
                self.send(&[0x03]);
                next_interrupt = Instant::now() + Duration::from_secs(1);
            }
            if Instant::now() >= deadline {
                panic!(
                    "TUI did not exit; screen:\n{}",
                    self.capture.lock().unwrap().screen()
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
        drop(std::mem::replace(
            &mut self.writer,
            Box::new(std::io::sink()),
        ));
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}

fn normalize_snapshot(mut screen: String, paths: &[String]) -> String {
    for path in paths {
        screen = screen.replace(path, "<FIXTURE>");
    }
    let screen = screen
        .lines()
        .map(|line| normalize_truncated_fixture_path(line, paths))
        .collect::<Vec<_>>()
        .join("\n");
    normalize_session_thread_ids(&screen)
}

fn normalize_truncated_fixture_path(line: &str, paths: &[String]) -> String {
    const MINIMUM_PREFIX_BYTES: usize = 24;

    let mut matched = None;
    for path in paths {
        for (end, _) in path.char_indices().rev() {
            if end < MINIMUM_PREFIX_BYTES {
                break;
            }
            let prefix = &path[..end];
            if line.contains(prefix) {
                if matched.is_none_or(|current: &str| prefix.len() > current.len()) {
                    matched = Some(prefix);
                }
                break;
            }
        }
    }
    let Some(prefix) = matched else {
        return line.to_string();
    };
    let replacement = format!(
        "<FIXTURE…>{}",
        " ".repeat(
            prefix
                .chars()
                .count()
                .saturating_sub("<FIXTURE…>".chars().count())
        )
    );
    line.replacen(prefix, &replacement, 1)
}

fn normalize_session_thread_ids(screen: &str) -> String {
    const PREFIX: &str = "thread:session-";
    const REPLACEMENT: &str = "thread:session-<ID>";

    let mut normalized = String::with_capacity(screen.len());
    let mut remaining = screen;
    while let Some(start) = remaining.find(PREFIX) {
        normalized.push_str(&remaining[..start]);
        let candidate = &remaining[start + PREFIX.len()..];
        let length = candidate
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || *byte == b'-')
            .count();
        if length == 0 {
            normalized.push_str(PREFIX);
            remaining = candidate;
        } else {
            normalized.push_str(REPLACEMENT);
            remaining = &candidate[length..];
        }
    }
    normalized.push_str(remaining);
    normalized
}

#[test]
fn snapshot_normalization_replaces_fixture_paths_and_generated_thread_ids() {
    let truncated = "/private/var/folders/account/T/zeta-fixt";
    let padding = " ".repeat(truncated.chars().count() - "<FIXTURE…>".chars().count());
    assert_eq!(
        normalize_snapshot(
            format!(
                "read /private/var/folders/account/T/zeta-fixture/workspace\n{truncated}\nthread:session-42-9001\nthread:session-label"
            ),
            &["/private/var/folders/account/T/zeta-fixture".into()],
        ),
        format!(
            "read <FIXTURE>/workspace\n<FIXTURE…>{padding}\nthread:session-<ID>\nthread:session-label"
        )
    );
}

#[test]
fn terminal_revision_advances_after_raw_capture_reaches_its_limit() {
    let mut capture = TerminalCapture::new(PtySize {
        rows: 1,
        cols: 1,
        pixel_width: 0,
        pixel_height: 0,
    });
    capture.raw.resize(OUTPUT_LIMIT, b'x');
    capture.revision = 41;

    capture.push(b"y");

    assert_eq!(capture.raw.len(), OUTPUT_LIMIT);
    assert_eq!(capture.revision(), 42);
}

fn assert_named_snapshot(name: &str, screen: String) {
    let name = Path::new(name);
    let snapshot_name = name
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| panic!("snapshot name must end in valid UTF-8: {}", name.display()));
    let mut snapshot_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    if let Some(parent) = name.parent() {
        snapshot_dir.push(parent);
    }

    let mut settings = insta::Settings::clone_current();
    settings.set_prepend_module_to_snapshot(false);
    settings.set_snapshot_path(snapshot_dir);
    settings.bind(|| insta::assert_snapshot!(snapshot_name, screen));
}

impl Drop for TuiProcess {
    fn drop(&mut self) {
        self.child.terminate();
    }
}

struct TerminalCapture {
    core: TerminalCore,
    raw: Vec<u8>,
    revision: u64,
    size: PtySize,
}

impl TerminalCapture {
    fn new(size: PtySize) -> Self {
        Self {
            core: TerminalCore::new(GridSize::new(size.rows, size.cols)),
            raw: Vec::new(),
            revision: 0,
            size,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.revision += 1;
        let remaining = OUTPUT_LIMIT.saturating_sub(self.raw.len());
        self.raw
            .extend_from_slice(&bytes[..bytes.len().min(remaining)]);
        self.core.process_output(bytes);
    }

    fn resize(&mut self, size: PtySize) {
        self.revision += 1;
        self.core.resize(GridSize::new(size.rows, size.cols));
        self.size = size;
    }

    fn screen(&self) -> String {
        self.core
            .grid()
            .lines()
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn raw_text(&self) -> String {
        String::from_utf8_lossy(&self.raw).into_owned()
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn screen_containing(&self, expected: &str) -> Option<String> {
        let mut core = TerminalCore::new(GridSize::new(self.size.rows, self.size.cols));
        let last = expected.as_bytes().last().copied()?;
        for byte in &self.raw {
            core.process_output(std::slice::from_ref(byte));
            if *byte == last {
                let screen = core
                    .grid()
                    .lines()
                    .iter()
                    .map(|line| line.text())
                    .collect::<Vec<_>>()
                    .join("\n");
                if screen.contains(expected) {
                    return Some(screen);
                }
            }
        }
        None
    }
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
