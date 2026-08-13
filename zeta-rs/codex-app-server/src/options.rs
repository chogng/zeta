use std::path::PathBuf;
use std::time::Duration;

/// Process inputs for one lazily started upstream Codex App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexAppServerOptions {
    pub(crate) program: PathBuf,
    pub(crate) request_timeout: Duration,
}

impl CodexAppServerOptions {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            request_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

impl Default for CodexAppServerOptions {
    fn default() -> Self {
        Self::new("codex")
    }
}
