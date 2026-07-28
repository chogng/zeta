use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TestRepository {
    root: PathBuf,
}

impl TestRepository {
    pub(crate) fn init() -> Self {
        let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "zeta-git-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create test repository");
        let root = std::fs::canonicalize(root).expect("canonicalize test repository");
        let repository = Self { root };
        repository.git(&["init", "--initial-branch=main"]);
        repository.git(&["config", "user.name", "Zeta Test"]);
        repository.git(&["config", "user.email", "zeta@example.invalid"]);
        repository.git(&["config", "commit.gpgsign", "false"]);
        repository
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub(crate) fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create test file parent");
        }
        std::fs::write(path, contents).expect("write test file");
    }

    pub(crate) fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path(relative)).expect("read test file")
    }

    pub(crate) fn commit_all(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "-m", message]);
    }

    pub(crate) fn git(&self, args: &[&str]) -> String {
        self.git_raw(args).trim().to_string()
    }

    pub(crate) fn git_raw(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(&self.root)
            .args(["-c", disabled_hooks_config()])
            .args(args)
            .output()
            .expect("run test Git command");
        assert!(
            output.status.success(),
            "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Git test output is UTF-8")
    }
}

impl Drop for TestRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn disabled_hooks_config() -> &'static str {
    if cfg!(windows) {
        "core.hooksPath=NUL"
    } else {
        "core.hooksPath=/dev/null"
    }
}
