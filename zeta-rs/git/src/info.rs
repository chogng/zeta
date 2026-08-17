use std::ffi::OsString;
use std::num::NonZeroUsize;

use crate::GitClient;
use crate::GitError;
use crate::GitRepository;
use crate::GitResult;

/// One local branch and its optional configured upstream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitBranch {
    name: String,
    object_id: String,
    current: bool,
    upstream: Option<String>,
}

impl GitBranch {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    pub fn is_current(&self) -> bool {
        self.current
    }

    pub fn upstream(&self) -> Option<&str> {
        self.upstream.as_deref()
    }
}

/// Fetch and push URLs configured for one named remote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRemote {
    name: String,
    fetch_urls: Vec<String>,
    push_urls: Vec<String>,
}

impl GitRemote {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fetch_urls(&self) -> &[String] {
        &self.fetch_urls
    }

    pub fn push_urls(&self) -> &[String] {
        &self.push_urls
    }

    /// Returns the provider-neutral repository identity parsed from the configured URLs.
    ///
    /// The identity intentionally excludes credentials and the original URL. Callers that need
    /// to display or associate a remote should use this projection rather than forwarding raw
    /// Git configuration values across a process boundary.
    pub fn identity(&self) -> Option<GitRemoteIdentity> {
        self.fetch_urls
            .iter()
            .chain(self.push_urls.iter())
            .find_map(|url| parse_remote_identity(url))
    }
}

/// Provider-neutral repository identity derived from one Git remote URL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRemoteIdentity {
    host: String,
    owner: String,
    repository: String,
}

impl GitRemoteIdentity {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }

    pub fn provider(&self) -> GitRemoteProvider {
        match self.host.as_str() {
            "github.com" => GitRemoteProvider::Github,
            "gitlab.com" => GitRemoteProvider::Gitlab,
            "bitbucket.org" => GitRemoteProvider::Bitbucket,
            _ => GitRemoteProvider::Other,
        }
    }
}

/// Well-known provider classification for a Git remote identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitRemoteProvider {
    Github,
    Gitlab,
    Bitbucket,
    Other,
}

/// Minimal commit metadata suitable for history lists and pickers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitCommitSummary {
    object_id: String,
    parent_object_ids: Vec<String>,
    timestamp_seconds: i64,
    subject: String,
}

impl GitCommitSummary {
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    pub fn parent_object_ids(&self) -> &[String] {
        &self.parent_object_ids
    }

    pub fn timestamp_seconds(&self) -> i64 {
        self.timestamp_seconds
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl GitClient {
    pub async fn local_branches(&self, repository: &GitRepository) -> GitResult<Vec<GitBranch>> {
        let output = self
            .run_query(
                repository.worktree_root(),
                [
                    "for-each-ref",
                    "--format=%(refname:short)%00%(objectname)%00%(upstream:short)%00%(HEAD)",
                    "refs/heads",
                ],
            )
            .await?;
        parse_branches(&output.stdout, &output.command)
    }

    pub async fn remotes(&self, repository: &GitRepository) -> GitResult<Vec<GitRemote>> {
        let output = self
            .run_query(repository.worktree_root(), ["remote"])
            .await?;
        let remote_names = parse_lines(&output.stdout, &output.command, "remote name")?;
        let mut remotes = Vec::with_capacity(remote_names.len());
        for name in remote_names {
            let fetch_urls = self
                .remote_urls(repository, &name, RemoteUrlKind::Fetch)
                .await?;
            let push_urls = self
                .remote_urls(repository, &name, RemoteUrlKind::Push)
                .await?;
            remotes.push(GitRemote {
                name,
                fetch_urls,
                push_urls,
            });
        }
        Ok(remotes)
    }

    pub async fn recent_commits(
        &self,
        repository: &GitRepository,
        limit: NonZeroUsize,
    ) -> GitResult<Vec<GitCommitSummary>> {
        let limit_arg = format!("-n{}", limit.get());
        let output = self
            .run_query(
                repository.worktree_root(),
                [
                    OsString::from("log"),
                    OsString::from("-z"),
                    OsString::from("--format=%H%x00%P%x00%ct%x00%s"),
                    OsString::from(limit_arg),
                ],
            )
            .await?;
        parse_commits(&output.stdout, &output.command)
    }

    async fn remote_urls(
        &self,
        repository: &GitRepository,
        remote: &str,
        kind: RemoteUrlKind,
    ) -> GitResult<Vec<String>> {
        let mut args = vec![OsString::from("remote"), OsString::from("get-url")];
        if kind == RemoteUrlKind::Push {
            args.push(OsString::from("--push"));
        }
        args.extend([
            OsString::from("--all"),
            OsString::from("--"),
            OsString::from(remote),
        ]);
        let output = self.run_query(repository.worktree_root(), args).await?;
        parse_lines(&output.stdout, &output.command, "remote URL")
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RemoteUrlKind {
    Fetch,
    Push,
}

fn parse_branches(bytes: &[u8], command: &str) -> GitResult<Vec<GitBranch>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| GitError::invalid_output(command, "branch output was not UTF-8"))?;
    let mut branches = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let fields = line.split('\0').collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(GitError::invalid_output(
                command,
                format!("branch record had {} fields instead of 4", fields.len()),
            ));
        }
        if fields[0].is_empty() || fields[1].is_empty() {
            return Err(GitError::invalid_output(
                command,
                "branch record omitted name or object id",
            ));
        }
        branches.push(GitBranch {
            name: fields[0].to_string(),
            object_id: fields[1].to_string(),
            upstream: (!fields[2].is_empty()).then(|| fields[2].to_string()),
            current: fields[3] == "*",
        });
    }
    Ok(branches)
}

fn parse_lines(bytes: &[u8], command: &str, label: &str) -> GitResult<Vec<String>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| GitError::invalid_output(command, format!("{label} output was not UTF-8")))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn parse_remote_identity(url: &str) -> Option<GitRemoteIdentity> {
    let value = url.trim().trim_end_matches('/');
    let (host, path) = if let Some((_, authority_and_path)) = value.split_once("://") {
        let (authority, path) = authority_and_path
            .split_once('/')
            .unwrap_or((authority_and_path, ""));
        (remote_host(authority), path)
    } else if let Some((authority, path)) = value.split_once(':') {
        if authority.contains('/') {
            return None;
        }
        (remote_host(authority), path)
    } else {
        return None;
    };
    let host = host.trim().trim_matches(['[', ']']).to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    let path = path.split_once('#').map_or(path, |(path, _)| path);
    let mut segments = path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }
    let mut repository = segments.pop()?;
    if let Some(stripped) = repository.strip_suffix(".git") {
        repository = stripped.to_string();
    }
    let owner = segments.join("/");
    if owner.is_empty() || repository.is_empty() {
        return None;
    }
    Some(GitRemoteIdentity {
        host,
        owner,
        repository,
    })
}

fn remote_host(authority: &str) -> &str {
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host)
        .trim();
    if let Some(host) = authority
        .strip_prefix('[')
        .and_then(|value| value.split_once(']'))
    {
        return host.0;
    }
    authority
        .rsplit_once(':')
        .filter(|(_, port)| {
            !port.is_empty() && port.chars().all(|character| character.is_ascii_digit())
        })
        .map_or(authority, |(host, _)| host)
}

pub(crate) fn parse_commits(bytes: &[u8], command: &str) -> GitResult<Vec<GitCommitSummary>> {
    let mut fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    if fields.last().is_some_and(|field| field.is_empty()) {
        fields.pop();
    }
    if fields.len() % 4 != 0 {
        return Err(GitError::invalid_output(
            command,
            "commit output did not contain groups of four fields",
        ));
    }
    fields
        .chunks_exact(4)
        .map(|fields| {
            let object_id = utf8(fields[0], command, "commit object id")?.to_string();
            let parent_object_ids = utf8(fields[1], command, "commit parent object ids")?
                .split_whitespace()
                .map(str::to_string)
                .collect();
            let timestamp = utf8(fields[2], command, "commit timestamp")?;
            let timestamp_seconds = timestamp.parse().map_err(|_| {
                GitError::invalid_output(command, "commit timestamp was not an integer")
            })?;
            let subject = utf8(fields[3], command, "commit subject")?.to_string();
            Ok(GitCommitSummary {
                object_id,
                parent_object_ids,
                timestamp_seconds,
                subject,
            })
        })
        .collect()
}

fn utf8<'a>(value: &'a [u8], command: &str, label: &str) -> GitResult<&'a str> {
    std::str::from_utf8(value)
        .map_err(|_| GitError::invalid_output(command, format!("{label} was not UTF-8")))
}

#[cfg(test)]
#[path = "info_tests.rs"]
mod tests;
