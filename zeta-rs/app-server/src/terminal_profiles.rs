use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use zeta_app_server_protocol::protocol::terminal::{TerminalProfile, TerminalProfileSelection};

/// Frozen trusted shell catalog used by one local Terminal service.
pub(crate) struct TerminalProfileCatalog {
    environment: HashMap<String, String>,
    profiles: Vec<TerminalProfileSpec>,
}

impl TerminalProfileCatalog {
    pub(crate) fn discover() -> Self {
        let environment = terminal_environment();
        let profiles = discover_profiles(&environment);
        Self {
            environment,
            profiles,
        }
    }

    pub(crate) fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    pub(crate) fn list(&self) -> Vec<TerminalProfile> {
        self.profiles.iter().map(TerminalProfileSpec::dto).collect()
    }

    pub(crate) fn resolve(
        &self,
        selection: &TerminalProfileSelection,
    ) -> Option<&TerminalProfileSpec> {
        match selection {
            TerminalProfileSelection::Default => {
                self.profiles.iter().find(|profile| profile.is_default)
            }
            TerminalProfileSelection::Profile { profile_id } => self
                .profiles
                .iter()
                .find(|profile| profile.profile_id == *profile_id),
        }
    }
}

pub(crate) struct TerminalProfileSpec {
    pub(crate) profile_id: String,
    pub(crate) title: String,
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    is_default: bool,
}

impl TerminalProfileSpec {
    pub(crate) fn dto(&self) -> TerminalProfile {
        TerminalProfile {
            profile_id: self.profile_id.clone(),
            title: self.title.clone(),
            is_default: self.is_default,
        }
    }
}

fn discover_profiles(environment: &HashMap<String, String>) -> Vec<TerminalProfileSpec> {
    let default_program = default_shell(environment);
    let mut candidates = vec![profile_for_program(default_program, true)];
    candidates.extend(platform_profiles(environment));
    let mut paths = HashSet::new();
    candidates
        .into_iter()
        .filter(|profile| paths.insert(normalized_program(&profile.program)))
        .collect()
}

#[cfg(windows)]
fn platform_profiles(environment: &HashMap<String, String>) -> Vec<TerminalProfileSpec> {
    let mut profiles = Vec::new();
    if let Some(program) = resolve_on_path(environment, "pwsh.exe") {
        profiles.push(profile("powershell", "PowerShell", program));
    }
    if let Some(system_root) = environment_value(environment, "SYSTEMROOT") {
        let windows_powershell = Path::new(system_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if windows_powershell.is_file() {
            profiles.push(profile(
                "windows-powershell",
                "Windows PowerShell",
                windows_powershell,
            ));
        }
    }
    if let Some(comspec) = environment_value(environment, "COMSPEC") {
        profiles.push(profile(
            "command-prompt",
            "Command Prompt",
            PathBuf::from(comspec),
        ));
    }
    for root_key in ["PROGRAMFILES", "LOCALAPPDATA"] {
        let Some(root) = environment_value(environment, root_key) else {
            continue;
        };
        let git_bash = if root_key == "PROGRAMFILES" {
            Path::new(root).join("Git").join("bin").join("bash.exe")
        } else {
            Path::new(root)
                .join("Programs")
                .join("Git")
                .join("bin")
                .join("bash.exe")
        };
        if git_bash.is_file() {
            profiles.push(profile("git-bash", "Git Bash", git_bash));
        }
    }
    if let Some(program) = resolve_on_path(environment, "zsh.exe") {
        profiles.push(profile("zsh", "Zsh", program));
    }
    profiles
}

#[cfg(not(windows))]
fn platform_profiles(environment: &HashMap<String, String>) -> Vec<TerminalProfileSpec> {
    [
        ("bash", "Bash", "bash"),
        ("zsh", "Zsh", "zsh"),
        ("fish", "Fish", "fish"),
        ("sh", "Shell", "sh"),
    ]
    .into_iter()
    .filter_map(|(profile_id, title, executable)| {
        resolve_on_path(environment, executable).map(|program| profile(profile_id, title, program))
    })
    .collect()
}

fn profile(
    profile_id: impl Into<String>,
    title: impl Into<String>,
    program: PathBuf,
) -> TerminalProfileSpec {
    TerminalProfileSpec {
        profile_id: profile_id.into(),
        title: title.into(),
        program: program.to_string_lossy().into_owned(),
        args: Vec::new(),
        is_default: false,
    }
}

fn profile_for_program(program: String, is_default: bool) -> TerminalProfileSpec {
    let executable = Path::new(&program)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("shell")
        .to_ascii_lowercase();
    let (profile_id, title) = match executable.as_str() {
        "cmd" => ("command-prompt", "Command Prompt"),
        "powershell" => ("windows-powershell", "Windows PowerShell"),
        "pwsh" => ("powershell", "PowerShell"),
        "bash" => ("bash", "Bash"),
        "zsh" => ("zsh", "Zsh"),
        "fish" => ("fish", "Fish"),
        _ => ("default", "Default Shell"),
    };
    TerminalProfileSpec {
        profile_id: profile_id.into(),
        title: title.into(),
        program,
        args: Vec::new(),
        is_default,
    }
}

fn resolve_on_path(environment: &HashMap<String, String>, executable: &str) -> Option<PathBuf> {
    let path = environment_value(environment, "PATH")?;
    std::env::split_paths(path)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

fn environment_value<'a>(environment: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    environment
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.as_str())
}

fn normalized_program(program: &str) -> String {
    #[cfg(windows)]
    {
        program.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        program.to_owned()
    }
}

fn terminal_environment() -> HashMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| allowed_environment_key(key))
        .chain([
            ("TERM".to_owned(), "xterm-256color".to_owned()),
            ("COLORTERM".to_owned(), "truecolor".to_owned()),
        ])
        .collect()
}

fn allowed_environment_key(key: &str) -> bool {
    matches!(
        key.to_ascii_uppercase().as_str(),
        "COMSPEC"
            | "HOME"
            | "HOMEDRIVE"
            | "HOMEPATH"
            | "LANG"
            | "LOCALAPPDATA"
            | "LOGNAME"
            | "PATH"
            | "PATHEXT"
            | "PROGRAMDATA"
            | "PROGRAMFILES"
            | "PROGRAMFILES(X86)"
            | "PSMODULEPATH"
            | "SHELL"
            | "SYSTEMDRIVE"
            | "SYSTEMROOT"
            | "TEMP"
            | "TMP"
            | "USER"
            | "USERDOMAIN"
            | "USERNAME"
            | "USERPROFILE"
            | "WINDIR"
    ) || key.to_ascii_uppercase().starts_with("LC_")
}

#[cfg(windows)]
fn default_shell(environment: &HashMap<String, String>) -> String {
    environment_value(environment, "COMSPEC")
        .map(str::to_owned)
        .unwrap_or_else(|| "cmd.exe".into())
}

#[cfg(not(windows))]
fn default_shell(environment: &HashMap<String, String>) -> String {
    environment
        .get("SHELL")
        .filter(|shell| shell.starts_with('/'))
        .cloned()
        .unwrap_or_else(|| "/bin/sh".into())
}

#[cfg(test)]
#[path = "terminal_profiles_tests.rs"]
mod tests;
