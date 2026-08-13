use std::path::{Path, PathBuf};

use zeta_input_classifier::ShellCommandEvidence;

const TASK_RUNNERS: &[(&str, &[&str])] = &[
    ("just", &["justfile", "Justfile", ".justfile"]),
    ("make", &["Makefile", "makefile", "GNUmakefile"]),
    ("cargo", &["Cargo.toml"]),
    ("npm", &["package.json"]),
    ("pnpm", &["package.json", "pnpm-workspace.yaml"]),
    ("yarn", &["package.json", "yarn.lock"]),
    ("bun", &["package.json", "bun.lock", "bun.lockb"]),
];

#[derive(Clone, Debug)]
pub(crate) struct ComposerShellDetector {
    working_directory: PathBuf,
    path_entries: Vec<PathBuf>,
}

impl ComposerShellDetector {
    pub(crate) fn new(working_directory: impl Into<PathBuf>) -> Self {
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

    pub(crate) fn set_working_directory(&mut self, working_directory: impl Into<PathBuf>) {
        self.working_directory = working_directory.into();
    }

    pub(crate) fn evidence(&self, text: &str) -> ShellCommandEvidence {
        let text = text.trim_start();
        if text.is_empty() || text.starts_with('/') {
            return ShellCommandEvidence::Absent;
        }
        let Some(command) = command_word(text) else {
            return ShellCommandEvidence::Absent;
        };
        let explicit_path = command.contains('/') || command.contains('\\');
        let short_input = text.split_whitespace().count() < 3;
        if (explicit_path && self.resolves_executable(command))
            || (short_input
                && (self.workspace_declares_runner(command) || self.resolves_executable(command)))
        {
            ShellCommandEvidence::HighConfidence
        } else {
            ShellCommandEvidence::Absent
        }
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
}

fn command_word(text: &str) -> Option<&str> {
    let end = text
        .char_indices()
        .find_map(|(index, character)| {
            (character.is_whitespace() || is_shell_operator(character)).then_some(index)
        })
        .unwrap_or(text.len());
    let word = &text[..end];
    (!word.is_empty() && !word.starts_with(['\'', '"'])).then_some(word)
}

fn is_shell_operator(character: char) -> bool {
    matches!(character, '|' | '&' | ';' | '<' | '>' | '$' | '`')
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
#[path = "composer_shell_tests.rs"]
mod tests;
