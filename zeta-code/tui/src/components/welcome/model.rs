use std::path::Path;

/// Display-only context for the empty-Thread welcome banner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WelcomeModel {
    directory: String,
}

impl WelcomeModel {
    pub(crate) fn for_dir(dir_root: &Path) -> Self {
        Self {
            directory: format_directory(dir_root, dirs::home_dir().as_deref()),
        }
    }

    pub(crate) fn directory(&self) -> &str {
        &self.directory
    }
}

fn format_directory(directory: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(relative) = directory.strip_prefix(home)
    {
        return if relative.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~{}{}", std::path::MAIN_SEPARATOR, relative.display())
        };
    }
    directory.display().to_string()
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
