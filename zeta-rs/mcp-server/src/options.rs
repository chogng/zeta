use crate::agent::RuntimeLimits;
use std::collections::BTreeSet;
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_TURN_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TURN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Host-owned filesystem and execution limits for one MCP server process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerOptions {
    profile_root: PathBuf,
    workspace_root: PathBuf,
    default_turn_timeout: Duration,
    maximum_turn_timeout: Duration,
    poll_interval: Duration,
}

/// Authenticated Streamable HTTP listener configuration.
#[derive(Clone, Eq, PartialEq)]
pub struct HttpServerOptions {
    listen_address: SocketAddr,
    endpoint_path: String,
    bearer_token: String,
    allowed_origins: BTreeSet<String>,
    maximum_connections: usize,
}

impl HttpServerOptions {
    pub fn new(
        listen_address: SocketAddr,
        endpoint_path: impl Into<String>,
        bearer_token: impl Into<String>,
    ) -> Self {
        Self {
            listen_address,
            endpoint_path: endpoint_path.into(),
            bearer_token: bearer_token.into(),
            allowed_origins: BTreeSet::new(),
            maximum_connections: 64,
        }
    }

    pub fn with_allowed_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.insert(origin.into());
        self
    }

    pub fn with_maximum_connections(mut self, maximum: usize) -> Self {
        self.maximum_connections = maximum;
        self
    }

    pub fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    pub fn endpoint_path(&self) -> &str {
        &self.endpoint_path
    }

    pub(crate) fn bearer_token(&self) -> &str {
        &self.bearer_token
    }

    pub(crate) fn allowed_origins(&self) -> &BTreeSet<String> {
        &self.allowed_origins
    }

    pub(crate) fn maximum_connections(&self) -> usize {
        self.maximum_connections
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.endpoint_path.starts_with('/')
            || self.endpoint_path.contains('?')
            || self.endpoint_path.contains('#')
        {
            return Err(
                "HTTP endpoint path must be an absolute path without query or fragment".into(),
            );
        }
        if self.bearer_token.len() < 32
            || !self
                .bearer_token
                .bytes()
                .all(|byte| byte.is_ascii_graphic())
        {
            return Err("HTTP bearer token must contain at least 32 visible ASCII bytes".into());
        }
        if self.maximum_connections == 0 {
            return Err("maximum HTTP connections must be greater than zero".into());
        }
        if self
            .allowed_origins
            .iter()
            .any(|origin| origin.is_empty() || !origin.is_ascii())
        {
            return Err("allowed HTTP origins must be non-empty ASCII values".into());
        }
        Ok(())
    }
}

impl fmt::Debug for HttpServerOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpServerOptions")
            .field("listen_address", &self.listen_address)
            .field("endpoint_path", &self.endpoint_path)
            .field("bearer_token", &"[redacted]")
            .field("allowed_origins", &self.allowed_origins)
            .field("maximum_connections", &self.maximum_connections)
            .finish()
    }
}

impl McpServerOptions {
    pub fn new(profile_root: impl Into<PathBuf>, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            profile_root: profile_root.into(),
            workspace_root: workspace_root.into(),
            default_turn_timeout: DEFAULT_TURN_TIMEOUT,
            maximum_turn_timeout: MAX_TURN_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }

    pub fn with_default_turn_timeout(mut self, timeout: Duration) -> Self {
        self.default_turn_timeout = timeout;
        self
    }

    pub fn with_maximum_turn_timeout(mut self, timeout: Duration) -> Self {
        self.maximum_turn_timeout = timeout;
        self
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    pub fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub(crate) fn runtime_limits(&self) -> RuntimeLimits {
        RuntimeLimits {
            default_turn_timeout: self.default_turn_timeout,
            maximum_turn_timeout: self.maximum_turn_timeout,
            poll_interval: self.poll_interval,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.default_turn_timeout.is_zero() {
            return Err("default Turn timeout must be greater than zero".into());
        }
        if self.maximum_turn_timeout.is_zero() {
            return Err("maximum Turn timeout must be greater than zero".into());
        }
        if self.default_turn_timeout > self.maximum_turn_timeout {
            return Err("default Turn timeout must not exceed the maximum".into());
        }
        if self.poll_interval.is_zero() {
            return Err("poll interval must be greater than zero".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
