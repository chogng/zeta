use std::path::Path;
use std::path::PathBuf;

use crate::parser::parse_shell_tokens;
use crate::rules::is_shell_command_keyword;

const TASK_RUNNERS: &[(&str, &[&str])] = &[
    ("just", &["justfile", "Justfile", ".justfile"]),
    ("make", &["Makefile", "makefile", "GNUmakefile"]),
    ("cargo", &["Cargo.toml"]),
    ("npm", &["package.json"]),
    ("pnpm", &["package.json", "pnpm-workspace.yaml"]),
    ("yarn", &["package.json", "yarn.lock"]),
    ("bun", &["package.json", "bun.lock", "bun.lockb"]),
];

const SHELL_BUILTINS: &[&str] = &[
    ".", "alias", "bg", "bind", "break", "builtin", "cd", "command", "continue", "declare", "dirs",
    "disown", "echo", "enable", "eval", "exec", "exit", "export", "false", "fc", "fg", "getopts",
    "hash", "help", "history", "jobs", "kill", "let", "local", "logout", "mapfile", "popd",
    "printf", "pushd", "pwd", "read", "readonly", "return", "set", "shift", "shopt", "source",
    "suspend", "test", "times", "trap", "true", "type", "typeset", "ulimit", "umask", "unalias",
    "unset", "wait",
];

const COMMON_SUBCOMMANDS: &[&str] = &[
    "add",
    "apply",
    "build",
    "checkout",
    "check",
    "clean",
    "clone",
    "commit",
    "config",
    "create",
    "delete",
    "diff",
    "doctor",
    "down",
    "exec",
    "fetch",
    "fmt",
    "get",
    "help",
    "init",
    "install",
    "list",
    "log",
    "merge",
    "new",
    "publish",
    "pull",
    "push",
    "rebase",
    "remove",
    "reset",
    "restore",
    "run",
    "search",
    "show",
    "start",
    "status",
    "stop",
    "switch",
    "test",
    "uninstall",
    "update",
    "up",
    "version",
];

#[derive(Clone, Debug)]
pub(super) struct ShellContext {
    working_directory: PathBuf,
    path_entries: Vec<PathBuf>,
}

impl ShellContext {
    pub(super) fn new(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            working_directory: working_directory.into(),
            path_entries: std::env::var_os("PATH")
                .as_deref()
                .map(std::env::split_paths)
                .into_iter()
                .flatten()
                .collect(),
        }
    }

    pub(super) fn set_working_directory(&mut self, working_directory: &Path) {
        self.working_directory = working_directory.to_path_buf();
    }

    pub(super) fn analyze(&self, input: &str) -> ShellTokenSnapshot {
        let tokens = parse_shell_tokens(input);
        let mut command_is_known = false;
        let tokens = tokens
            .into_iter()
            .map(|token| {
                if token.token_index == 0 {
                    command_is_known = self.is_known_command(&token.text);
                }
                let described = if token.token_index == 0 {
                    command_is_known
                } else {
                    self.describes_argument(&token.text, command_is_known)
                };
                ShellToken {
                    text: token.text,
                    token_index: token.token_index,
                    described,
                }
            })
            .collect();
        ShellTokenSnapshot { tokens }
    }

    fn is_known_command(&self, command: &str) -> bool {
        let command = trim_shell_quotes(command);
        let normalized = command.to_lowercase();
        is_shell_command_keyword(&normalized)
            || SHELL_BUILTINS.contains(&normalized.as_str())
            || self.workspace_declares_runner(&normalized)
            || self.resolves_executable(command)
    }

    fn describes_argument(&self, argument: &str, command_is_known: bool) -> bool {
        let argument = trim_shell_quotes(argument);
        let normalized = argument.to_lowercase();
        (command_is_known
            && (argument.starts_with('-') || COMMON_SUBCOMMANDS.contains(&normalized.as_str())))
            || self.resolves_path(argument)
    }

    fn workspace_declares_runner(&self, command: &str) -> bool {
        TASK_RUNNERS
            .iter()
            .find(|(runner, _)| *runner == command)
            .is_some_and(|(_, manifests)| {
                self.working_directory
                    .ancestors()
                    .any(|directory| manifests.iter().any(|name| directory.join(name).is_file()))
            })
    }

    fn resolves_executable(&self, command: &str) -> bool {
        if command.contains('/') || command.contains('\\') {
            let path = Path::new(command);
            let candidate = if path.is_absolute() {
                path.to_path_buf()
            } else {
                self.working_directory.join(path)
            };
            return executable_file(&candidate);
        }
        self.path_entries
            .iter()
            .any(|directory| executable_candidate(directory, command))
    }

    fn resolves_path(&self, value: &str) -> bool {
        if value.is_empty() || value.contains(['*', '?', '$']) {
            return false;
        }
        let path = Path::new(value);
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.working_directory.join(path)
        };
        candidate.exists()
    }
}

#[derive(Clone, Debug)]
struct ShellToken {
    text: String,
    token_index: usize,
    described: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ShellTokenSnapshot {
    tokens: Vec<ShellToken>,
}

impl ShellTokenSnapshot {
    pub(super) fn is_likely_shell_command(&self, word_token_count: usize) -> bool {
        if self.tokens.is_empty() {
            return false;
        }
        if self.tokens.iter().any(|token| {
            token.token_index == 0
                && is_shell_command_keyword(&trim_shell_quotes(&token.text).to_lowercase())
        }) {
            return true;
        }
        let described_count = self.tokens.iter().filter(|token| token.described).count();
        let last_command_is_known = self
            .tokens
            .iter()
            .rev()
            .find(|token| token.token_index == 0)
            .is_some_and(|token| token.described);
        described_count == self.tokens.len() || (word_token_count < 3 && last_command_is_known)
    }

    pub(super) fn first_token_is_command(&self) -> bool {
        self.tokens.first().is_some_and(|token| token.described)
    }
}

fn trim_shell_quotes(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('`') && value.ends_with('`')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn executable_candidate(directory: &Path, command: &str) -> bool {
    if executable_file(&directory.join(command)) {
        return true;
    }
    #[cfg(windows)]
    {
        let extensions = std::env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned());
        return extensions
            .split(';')
            .any(|extension| executable_file(&directory.join(format!("{command}{extension}"))));
    }
    #[cfg(not(windows))]
    false
}

fn executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    true
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
