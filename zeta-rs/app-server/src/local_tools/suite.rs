use super::LocalShellToolService;
use super::SessionAdditionalDirectoryAccess;
use super::read_only_sandbox;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use zeta_action_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewRequest, ActionSource, Capability,
    CapabilityKind, CapabilitySet, ProcessInvocationKind, ResolvedAction, SandboxCompatibility,
};
use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, ToolAuthorization, ToolExecutionFacts, ToolOutputSink, ToolService};
use zeta_file_system::{FileWriteCondition, LocalFileSystem, WorkspaceFileSystem};
use zeta_protocol::SessionId;
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolName, ToolOutputStream};
use zeta_shell_command::RipgrepExecutable;
use zeta_workspace::TrustedWorkspace;
use zeta_workspace::WorkspaceRoot;

const READ_DESCRIPTION: &str = r#"Reads a file from the workspace and returns its content with line numbers.

Usage notes:
- Returns at most 2000 lines starting from `offset` (1-based). The last line
  of a truncated read says how many lines remain; call again with a larger
  offset to continue.
- Lines longer than 2000 characters are truncated with a marker.
- Binary files, including images, are rejected; use a dedicated viewer for images.
- You must read a file before editing or overwriting it.
- Prefer reading whole files (omit offset/limit) unless the file is too large."#;
const WRITE_DESCRIPTION: &str = r#"Creates or overwrites a file with the given content.

Usage notes:
- Overwriting an existing file you have not read in this conversation fails;
  read it first.
- Prefer apply_patch for modifying existing files, or edit for one small exact
  replacement; use write_file for new files or full rewrites you have read.
- Parent directories are created automatically.
- Never proactively create documentation files unless explicitly requested."#;
const EDIT_DESCRIPTION: &str = r#"Performs an exact string replacement in a file.

Usage notes:
- You must read the file first; the edit fails otherwise.
- Use edit for one small, exact replacement, or as a fallback when a narrow
  apply_patch context cannot match. Prefer apply_patch for coordinated changes
  across multiple locations or files.
- old_string must match the file content exactly, including whitespace and
  indentation, and must identify a unique location. If it matches more than
  one location, extend it with surrounding lines until unique, or set
  replace_all to true to change every occurrence.
- Do not include line-number prefixes from read_file output in old_string.
- For moves or renames use shell with git mv; for full rewrites use write_file."#;
const GREP_DESCRIPTION: &str = r#"Searches file contents with a regular expression (ripgrep syntax).

- Full regex support, e.g. "fn\\s+resolve" or "TODO|FIXME".
- Results are file:line:content, capped at 100 matches; narrow with `glob`
  or `path` if truncated.
- Literal braces or dots must be escaped: use "foo\\.bar" to match "foo.bar".
- Always use this tool instead of invoking grep or rg through shell."#;
const GLOB_DESCRIPTION: &str = r#"Finds files by glob pattern, sorted by most recently modified.

- Supports patterns like "**/*.rs" or "src/**/*.test.ts".
- Returns at most 100 paths; narrow the pattern if truncated.
- Use grep to search file contents; use glob to find files by name."#;

fn schema(value: &str) -> Value {
    serde_json::from_str(value).expect("static tool schema is valid")
}

fn definition(name: &str, description: &str, parameters: &str) -> ToolDefinition {
    ToolDefinition {
        name: ToolName::new(name).expect("static tool name is valid"),
        description: description.into(),
        parameters: schema(parameters),
        strict: true,
    }
}

const READ_SCHEMA: &str = r#"{"type":"object","properties":{"path":{"type":"string","description":"Absolute path to the file to read."},"offset":{"type":["integer","null"],"description":"1-based line number to start from. Defaults to 1."},"limit":{"type":["integer","null"],"description":"Maximum lines to return. Defaults to 2000."}},"required":["path","offset","limit"],"additionalProperties":false}"#;
const WRITE_SCHEMA: &str = r#"{"type":"object","properties":{"path":{"type":"string","description":"Absolute path of the file to write."},"content":{"type":"string","description":"Full content to write. The previous content is replaced entirely."}},"required":["path","content"],"additionalProperties":false}"#;
const EDIT_SCHEMA: &str = r#"{"type":"object","properties":{"path":{"type":"string","description":"Absolute path of the file to modify."},"old_string":{"type":"string","description":"Exact text to replace. Must be unique in the file unless replace_all is true."},"new_string":{"type":"string","description":"Replacement text. Must differ from old_string."},"replace_all":{"type":["boolean","null"],"description":"Replace every occurrence. Defaults to false."}},"required":["path","old_string","new_string","replace_all"],"additionalProperties":false}"#;
const GREP_SCHEMA: &str = r#"{"type":"object","properties":{"pattern":{"type":"string","description":"Regular expression to search for (ripgrep syntax)."},"path":{"type":["string","null"],"description":"File or directory to search. Defaults to the workspace root."},"glob":{"type":["string","null"],"description":"Restrict to files matching this glob, e.g. \"*.rs\"."},"case_insensitive":{"type":["boolean","null"],"description":"Case-insensitive search. Defaults to false."}},"required":["pattern","path","glob","case_insensitive"],"additionalProperties":false}"#;
const GLOB_SCHEMA: &str = r#"{"type":"object","properties":{"pattern":{"type":"string","description":"Glob pattern to match file paths against."},"path":{"type":["string","null"],"description":"Directory to search in. Defaults to the workspace root."}},"required":["pattern","path"],"additionalProperties":false}"#;
const MAX_READ_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_WRITE_FILE_BYTES: usize = 10 * 1024 * 1024;
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);

/// The fixed local coding-tool suite and its path-scoped execution state.
pub(crate) struct LocalToolSuite<B> {
    shell: LocalShellToolService<B>,
    ripgrep: RipgrepExecutable,
    workspace: TrustedWorkspace,
    additional_directories: Arc<SessionAdditionalDirectoryAccess>,
    read_paths: Mutex<BTreeSet<(String, PathBuf)>>,
    read_fingerprints: Mutex<std::collections::BTreeMap<(String, PathBuf), String>>,
    definitions: Vec<ToolDefinition>,
}

struct ResolvedFilePath {
    root: WorkspaceRoot,
    relative: PathBuf,
    absolute: PathBuf,
}

impl<B: zeta_sandboxing::SandboxBackend> LocalToolSuite<B> {
    pub(super) fn new(
        shell: LocalShellToolService<B>,
        ripgrep: RipgrepExecutable,
        additional_directories: Arc<SessionAdditionalDirectoryAccess>,
    ) -> Self {
        let workspace = shell.workspace.clone();
        let definitions = vec![
            shell.definition.clone(),
            definition("read_file", READ_DESCRIPTION, READ_SCHEMA),
            definition("write_file", WRITE_DESCRIPTION, WRITE_SCHEMA),
            definition("edit", EDIT_DESCRIPTION, EDIT_SCHEMA),
            definition("grep", GREP_DESCRIPTION, GREP_SCHEMA),
            definition("glob", GLOB_DESCRIPTION, GLOB_SCHEMA),
        ];
        Self {
            shell,
            ripgrep,
            workspace,
            additional_directories,
            read_paths: Mutex::new(BTreeSet::new()),
            read_fingerprints: Mutex::new(std::collections::BTreeMap::new()),
            definitions,
        }
    }

    fn resolve(
        &self,
        value: &str,
        existing: bool,
        session_id: Option<&SessionId>,
    ) -> Result<ResolvedFilePath, String> {
        self.workspace
            .ensure_active()
            .map_err(|error| error.to_string())?;
        let path = PathBuf::from(value);
        let mut roots = vec![self.workspace.root().clone()];
        if let Some(session_id) = session_id {
            roots.extend(
                self.additional_directories
                    .roots(session_id)
                    .into_iter()
                    .filter(|workspace| workspace.ensure_active().is_ok())
                    .map(|workspace| workspace.root().clone()),
            );
        }
        let (root, relative) = if path.is_absolute() {
            roots
                .into_iter()
                .filter_map(|root| {
                    path.strip_prefix(root.canonical_path())
                        .or_else(|_| path.strip_prefix(root.requested_path()))
                        .ok()
                        .map(|relative| (root, relative.to_path_buf()))
                })
                .max_by_key(|(root, _)| root.canonical_path().components().count())
                .ok_or_else(|| {
                    format!("path is outside the workspace and added directories: {value}")
                })?
        } else {
            (self.workspace.root().clone(), path)
        };
        let absolute = if relative.as_os_str().is_empty() {
            Ok(root.canonical_path().to_path_buf())
        } else if existing {
            root.resolve_existing(&relative)
        } else {
            root.resolve_for_write(&relative)
        }
        .map_err(|_| format!("path is outside the workspace: {value}"))?;
        Ok(ResolvedFilePath {
            root,
            relative,
            absolute,
        })
    }

    fn review(
        &self,
        call: &ToolCall,
        write: bool,
        session_id: Option<&SessionId>,
    ) -> Result<ActionReviewRequest, CoreError> {
        let path = call
            .arguments
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_else(|| {
                self.workspace
                    .root()
                    .canonical_path()
                    .to_str()
                    .unwrap_or(".")
            });
        let resolved = self
            .resolve(path, false, session_id)
            .map_err(CoreError::Policy)?;
        let source_id = call.name.as_str();
        let canonical = serde_json::to_vec(
            &json!({"tool": source_id, "path": resolved.absolute, "arguments": call.arguments}),
        )
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let capabilities = CapabilitySet::new([
            Capability::new(
                CapabilityKind::FileRead,
                resolved.root.canonical_path().display().to_string(),
            ),
            if write {
                Capability::new(
                    CapabilityKind::FileWrite,
                    resolved.absolute.display().to_string(),
                )
            } else {
                Capability::new(
                    CapabilityKind::ProcessSpawn,
                    self.ripgrep.path().display().to_string(),
                )
            },
        ]);
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                if write {
                    ActionKind::FileSystemMutation
                } else {
                    ActionKind::LocalProcess(ProcessInvocationKind::Direct)
                },
                format!("{} {}", source_id, resolved.absolute.display()),
                capabilities,
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, source_id),
            if write {
                SandboxCompatibility::NotApplicable {
                    reason: "file mutation approval is host-mediated".into(),
                }
            } else {
                SandboxCompatibility::Supported(read_only_sandbox())
            },
            self.shell.action_policy_revision.clone(),
        ))
    }

    fn read_file(
        &self,
        call: &ToolCall,
        scope: &str,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let path = string_arg(&call.arguments, "path")?;
        let resolved = self
            .resolve(&path, false, session_id)
            .map_err(CoreError::Execution)?;
        let metadata = fs::metadata(&resolved.absolute)
            .map_err(|_| CoreError::Execution(format!("file not found: {path}")))?;
        if metadata.is_dir() {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} is a directory. Use glob to list its files"
            )));
        }
        if metadata.len() > MAX_READ_FILE_BYTES {
            return Ok(ToolExecutionOutput::Failure(format!(
                "file too large to read: {path} exceeds 10485760 bytes"
            )));
        }
        let bytes = fs::read(&resolved.absolute)
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        let text = match String::from_utf8(bytes) {
            Ok(text) if !text.as_bytes().contains(&0) => text,
            _ => {
                return Ok(ToolExecutionOutput::Failure(format!(
                    "{path} is a binary file and cannot be displayed as text"
                )));
            }
        };
        self.read_paths
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((scope.into(), resolved.absolute.clone()));
        self.read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                (scope.into(), resolved.absolute.clone()),
                format!("{:x}", Sha256::digest(text.as_bytes())),
            );
        if text.is_empty() {
            return Ok(ToolExecutionOutput::Success("(file is empty)".into()));
        }
        let lines = text.lines().collect::<Vec<_>>();
        let offset = nullable_u64(&call.arguments, "offset")?.unwrap_or(1);
        let limit = nullable_u64(&call.arguments, "limit")?.unwrap_or(2000);
        if offset == 0 || limit == 0 {
            return Err(CoreError::Execution(
                "offset and limit must be greater than zero".into(),
            ));
        }
        let start = (offset - 1) as usize;
        let end = start.saturating_add(limit as usize).min(lines.len());
        let mut output = lines[start.min(lines.len())..end]
            .iter()
            .enumerate()
            .map(|(index, line)| format!("{:>6}\t{}", start + index + 1, truncate_line(line)))
            .collect::<Vec<_>>();
        if end < lines.len() {
            output.push(format!(
                "[... {} more lines, continue with offset={}]",
                lines.len() - end,
                end + 1
            ));
        }
        Ok(ToolExecutionOutput::Success(output.join("\n")))
    }

    fn write_file(
        &self,
        call: &ToolCall,
        scope: &str,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let path = string_arg(&call.arguments, "path")?;
        let content = string_arg(&call.arguments, "content")?;
        let resolved = self
            .resolve(&path, false, session_id)
            .map_err(CoreError::Execution)?;
        if resolved.absolute.is_dir() {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} is a directory"
            )));
        }
        if resolved.absolute.exists()
            && !self
                .read_paths
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(&(scope.into(), resolved.absolute.clone()))
        {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} exists but has not been read in this conversation. Read it first, or choose a new path"
            )));
        }
        if let Some(parent) = resolved.absolute.parent() {
            fs::create_dir_all(parent).map_err(|error| CoreError::Execution(error.to_string()))?;
        }
        let expected_revision = self
            .read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(scope.into(), resolved.absolute.clone()))
            .cloned();
        if resolved.absolute.exists() && expected_revision.is_none() {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} must be read again after reconnecting before it can be overwritten"
            )));
        }
        let file_system = LocalFileSystem::new(resolved.root.clone());
        let write = match expected_revision {
            Some(revision) => file_system.write_file_with_condition(
                &resolved.relative,
                content.as_bytes(),
                MAX_WRITE_FILE_BYTES,
                &FileWriteCondition::ExpectedRevision(revision),
            ),
            None => {
                file_system.write_file(&resolved.relative, content.as_bytes(), MAX_WRITE_FILE_BYTES)
            }
        };
        write.map_err(|error| CoreError::Execution(error.to_string()))?;
        self.read_paths
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((scope.into(), resolved.absolute.clone()));
        self.read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                (scope.into(), resolved.absolute),
                format!("{:x}", Sha256::digest(content.as_bytes())),
            );
        Ok(ToolExecutionOutput::Success(format!("wrote {path}")))
    }

    fn edit(
        &self,
        call: &ToolCall,
        scope: &str,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let path = string_arg(&call.arguments, "path")?;
        let old = string_arg(&call.arguments, "old_string")?;
        let new = string_arg(&call.arguments, "new_string")?;
        let replace_all = nullable_bool(&call.arguments, "replace_all")?.unwrap_or(false);
        if old == new {
            return Ok(ToolExecutionOutput::Failure(
                "new_string must differ from old_string".into(),
            ));
        }
        let resolved = self
            .resolve(&path, false, session_id)
            .map_err(CoreError::Execution)?;
        if !self
            .read_paths
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&(scope.into(), resolved.absolute.clone()))
        {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} has not been read in this conversation. Read it first"
            )));
        }
        let text = fs::read_to_string(&resolved.absolute)
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        if let Some(expected) = self
            .read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(scope.into(), resolved.absolute.clone()))
            && expected != &format!("{:x}", Sha256::digest(text.as_bytes()))
        {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} changed on disk after your last read. Read it again before editing"
            )));
        }
        let count = text.match_indices(&old).count();
        if count == 0 {
            return Ok(ToolExecutionOutput::Failure(format!(
                "old_string not found in {path}. Re-read the file: the content may differ from what you expect (check whitespace and indentation)"
            )));
        }
        if count > 1 && !replace_all {
            return Ok(ToolExecutionOutput::Failure(format!(
                "old_string matches {count} locations in {path}. Extend it with more surrounding context to make it unique, or set replace_all to true"
            )));
        }
        let replacement_line = text[..text.find(&old).expect("count is non-zero")]
            .lines()
            .count()
            .max(1);
        let replaced = if replace_all {
            text.replace(&old, &new)
        } else {
            text.replacen(&old, &new, 1)
        };
        let Some(expected_revision) = self
            .read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(scope.into(), resolved.absolute.clone()))
            .cloned()
        else {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{path} must be read again after reconnecting before it can be edited"
            )));
        };
        LocalFileSystem::new(resolved.root.clone())
            .write_file_with_condition(
                &resolved.relative,
                replaced.as_bytes(),
                MAX_WRITE_FILE_BYTES,
                &FileWriteCondition::ExpectedRevision(expected_revision),
            )
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        self.read_fingerprints
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                (scope.into(), resolved.absolute),
                format!("{:x}", Sha256::digest(replaced.as_bytes())),
            );
        let lines = replaced.lines().collect::<Vec<_>>();
        let excerpt_start = replacement_line.saturating_sub(5);
        let excerpt_end = excerpt_start.saturating_add(9).min(lines.len());
        let excerpt = lines[excerpt_start..excerpt_end]
            .iter()
            .enumerate()
            .map(|(index, line)| format!("{:>6}\t{}", excerpt_start + index + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolExecutionOutput::Success(excerpt))
    }

    fn grep(
        &self,
        call: &ToolCall,
        cancellation: &CancellationToken,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let pattern = string_arg(&call.arguments, "pattern")?;
        let path = nullable_string(&call.arguments, "path")?
            .unwrap_or_else(|| self.workspace.root().canonical_path().display().to_string());
        let resolved = self
            .resolve(&path, false, session_id)
            .map_err(CoreError::Execution)?;
        let glob = nullable_string(&call.arguments, "glob")?;
        let insensitive = nullable_bool(&call.arguments, "case_insensitive")?.unwrap_or(false);
        let mut command = Command::new(self.ripgrep.path());
        command.args(["--no-config", "-n", "--no-heading"]);
        if insensitive {
            command.arg("-i");
        }
        if let Some(glob) = glob {
            command.args(["--glob", &glob]);
        }
        command.arg("--").arg(pattern).arg(resolved.absolute);
        let output = match run_search(command, cancellation) {
            Ok(output) => output,
            Err(SearchError::Cancelled(error)) => return Err(error),
            Err(SearchError::Failed(message)) => return Ok(ToolExecutionOutput::Failure(message)),
        };
        if output.status.code() == Some(1) {
            return Ok(ToolExecutionOutput::Success("no matches".into()));
        }
        if !output.status.success() {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{}\nescape literal characters like . ( ) {{ }} with a backslash",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(ToolExecutionOutput::Success(limit_matches(
            &String::from_utf8_lossy(&output.stdout),
            500,
        )))
    }

    fn glob(
        &self,
        call: &ToolCall,
        cancellation: &CancellationToken,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let pattern = string_arg(&call.arguments, "pattern")?;
        let path = nullable_string(&call.arguments, "path")?
            .unwrap_or_else(|| self.workspace.root().canonical_path().display().to_string());
        let resolved = self
            .resolve(&path, false, session_id)
            .map_err(CoreError::Execution)?;
        let mut command = Command::new(self.ripgrep.path());
        command
            .args(["--no-config", "--files", "--glob", &pattern])
            .arg(resolved.absolute);
        let output = match run_search(command, cancellation) {
            Ok(output) => output,
            Err(SearchError::Cancelled(error)) => return Err(error),
            Err(SearchError::Failed(message)) => return Ok(ToolExecutionOutput::Failure(message)),
        };
        if !output.status.success() {
            let reason = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if reason.is_empty() {
                return Ok(ToolExecutionOutput::Success(format!(
                    "no files match {pattern}"
                )));
            }
            return Ok(ToolExecutionOutput::Failure(format!(
                "invalid glob pattern: {reason}"
            )));
        }
        let mut paths = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        paths.sort_by_key(|path| {
            std::cmp::Reverse(
                fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok(),
            )
        });
        let total = paths.len();
        paths.truncate(100);
        let mut text = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        if total > 100 {
            text.push_str(&format!("\n[{total} matches, showing first 100]"));
        }
        if text.is_empty() {
            text = format!("no files match {pattern}");
        }
        Ok(ToolExecutionOutput::Success(text))
    }
}

impl<B: zeta_sandboxing::SandboxBackend> ToolService for LocalToolSuite<B> {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        if call.name.as_str() == "shell-command" {
            return self.shell.prepare(call);
        }
        match call.name.as_str() {
            "read_file" | "grep" | "glob" => self.review(call, false, None),
            "write_file" | "edit" => self.review(call, true, None),
            _ => Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            ))),
        }
    }

    fn prepare_with_facts(
        &self,
        call: &ToolCall,
        facts: &ToolExecutionFacts,
    ) -> Result<ActionReviewRequest, CoreError> {
        if call.name.as_str() == "shell-command" {
            return self.shell.prepare(call);
        }
        let session_id = facts
            .execution_identity()
            .map(|identity| identity.session_id())
            .ok_or_else(|| {
                CoreError::Policy("local tools require durable caller identity".into())
            })?;
        match call.name.as_str() {
            "read_file" | "grep" | "glob" => self.review(call, false, Some(session_id)),
            "write_file" | "edit" => self.review(call, true, Some(session_id)),
            _ => Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            ))),
        }
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute_scoped(call, authorization, cancellation, "direct", None)
    }

    fn execute_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.execute_streaming_with_facts(
            call,
            authorization,
            cancellation,
            facts,
            &mut NoopToolOutputSink,
        )
    }

    fn execute_streaming_with_facts(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        facts: &ToolExecutionFacts,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let identity = facts.execution_identity().ok_or_else(|| {
            CoreError::Execution("local tools require durable caller identity".into())
        })?;
        let scope = identity.thread_id().to_string();
        let session_id = identity.session_id();
        for path in facts.read_paths() {
            if let Ok(resolved) = self.resolve(&path.display().to_string(), true, Some(session_id))
            {
                self.read_paths
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert((scope.clone(), resolved.absolute));
            }
        }
        let output =
            self.execute_scoped(call, authorization, cancellation, &scope, Some(session_id))?;
        if let ToolExecutionOutput::Success(text) = &output {
            sink.emit(ToolOutputStream::Stdout, text.clone())?;
        }
        Ok(output)
    }

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let output = self.execute_scoped(call, authorization, cancellation, "direct", None)?;
        if let ToolExecutionOutput::Success(text) = &output {
            sink.emit(ToolOutputStream::Stdout, text.clone())?;
        }
        Ok(output)
    }
}

impl<B: zeta_sandboxing::SandboxBackend> LocalToolSuite<B> {
    fn execute_scoped(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        scope: &str,
        session_id: Option<&SessionId>,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if call.name.as_str() == "shell-command" {
            return self.shell.execute(call, authorization, cancellation);
        }
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        match call.name.as_str() {
            "read_file" => self.read_file(call, scope, session_id),
            "write_file" => self.write_file(call, scope, session_id),
            "edit" => self.edit(call, scope, session_id),
            "grep" => self.grep(call, cancellation, session_id),
            "glob" => self.glob(call, cancellation, session_id),
            _ => Ok(ToolExecutionOutput::Failure(format!(
                "tool is not available: {}",
                call.name
            ))),
        }
    }
}

struct NoopToolOutputSink;

impl ToolOutputSink for NoopToolOutputSink {
    fn emit(&mut self, _: ToolOutputStream, _: String) -> Result<(), CoreError> {
        Ok(())
    }
}

enum SearchError {
    Cancelled(CoreError),
    Failed(String),
}

#[cfg(test)]
#[path = "suite_tests.rs"]
mod tests;

fn run_search(
    mut command: Command,
    cancellation: &CancellationToken,
) -> Result<Output, SearchError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| SearchError::Failed(format!("could not execute search: {error}")))?;
    let started = Instant::now();
    loop {
        cancellation.check().map_err(|signal| {
            SearchError::Cancelled(CoreError::Cancelled(signal.reason().to_string()))
        })?;
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| SearchError::Failed(error.to_string()));
            }
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SearchError::Failed(format!(
                    "could not wait for search: {error}"
                )));
            }
        }
        if started.elapsed() >= SEARCH_TIMEOUT {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| SearchError::Failed(error.to_string()))?;
            let partial = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(SearchError::Failed(format!(
                "search timed out after 30000 ms. Partial output:\n{}",
                partial
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn string_arg(arguments: &Value, name: &str) -> Result<String, CoreError> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| CoreError::Execution(format!("{name} must be a string")))
}

fn nullable_string(arguments: &Value, name: &str) -> Result<Option<String>, CoreError> {
    match arguments.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| CoreError::Execution(format!("{name} must be a string or null"))),
    }
}

fn nullable_u64(arguments: &Value, name: &str) -> Result<Option<u64>, CoreError> {
    match arguments.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| CoreError::Execution(format!("{name} must be an integer or null"))),
    }
}

fn nullable_bool(arguments: &Value, name: &str) -> Result<Option<bool>, CoreError> {
    match arguments.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| CoreError::Execution(format!("{name} must be a boolean or null"))),
    }
}

fn truncate_line(line: &str) -> String {
    if line.chars().count() <= 2000 {
        line.to_owned()
    } else {
        format!(
            "{} [... line truncated ...]",
            line.chars().take(2000).collect::<String>()
        )
    }
}

fn limit_matches(output: &str, line_limit: usize) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let total = lines.len();
    let mut result = lines
        .into_iter()
        .take(100)
        .map(|line| {
            if line.chars().count() > line_limit {
                format!(
                    "{} [... line truncated ...]",
                    line.chars().take(line_limit).collect::<String>()
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if total > 100 {
        result.push_str(&format!("\n[{total} matches, showing first 100]"));
    }
    result
}
