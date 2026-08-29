use std::fmt;
use std::path::PathBuf;

/// Selects whether a search query is interpreted literally or as a regular expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceSearchPattern {
    Literal,
    Regex,
}

/// Selects how character case is matched for one search query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceSearchCaseSensitivity {
    Smart,
    Sensitive,
    Insensitive,
}

/// Validated intent for one bounded content search.
///
/// [`WorkspaceSearchService::start`](crate::WorkspaceSearchService::start) validates the string and glob limits
/// before it starts a process. Callers may construct this value from their own protocol or UI
/// types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchQuery {
    pub query: String,
    pub pattern: WorkspaceSearchPattern,
    pub case_sensitivity: WorkspaceSearchCaseSensitivity,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub max_results: usize,
}

/// Opaque identity supplied by a host to isolate one caller's search jobs.
///
/// The search crate compares this value when a job is read or cancelled; it does not interpret
/// the value as an App Server connection, user, or session identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorkspaceSearchOwner(u64);

impl WorkspaceSearchOwner {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// UTF-16 offsets for one match within a returned preview line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchMatchRange {
    pub start: usize,
    pub end: usize,
}

/// One matching line in a workspace-relative file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub preview: String,
    pub ranges: Vec<WorkspaceSearchMatchRange>,
}

/// One bounded read of a running or completed search job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchPage {
    pub matches: Vec<WorkspaceSearchMatch>,
    pub next_match: usize,
    pub completed: bool,
    pub limit_hit: bool,
    pub error: Option<String>,
}

/// Failure while validating, locating, reading, or cancelling a search job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceSearchError {
    InvalidInput,
    NotFound,
    NotOwner,
    Busy,
    Unavailable,
}

impl fmt::Display for WorkspaceSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("search input is invalid"),
            Self::NotFound => formatter.write_str("search job was not found"),
            Self::NotOwner => formatter.write_str("search job belongs to another owner"),
            Self::Busy => formatter.write_str("search is busy"),
            Self::Unavailable => formatter.write_str("search workspace is unavailable"),
        }
    }
}

impl std::error::Error for WorkspaceSearchError {}
