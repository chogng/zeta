use std::path::Path;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;

const SEPARATOR: &str = " · ";

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisplayValue {
    full: String,
    compact: String,
}

/// Pure display projection for the configurable context row above the composer.
///
/// Data acquisition remains with the application and the typed interfaces that own each value.
/// This model only keeps display variants and selects the richest representation that fits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatusLineModel {
    preferred_model: Option<DisplayValue>,
    workspace: DisplayValue,
}

impl StatusLineModel {
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        let full = workspace_root.display().to_string();
        let compact = workspace_root
            .file_name()
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| full.clone());
        Self {
            preferred_model: None,
            workspace: DisplayValue { full, compact },
        }
    }

    pub(crate) fn apply_config(&mut self, config: &ConfigReadResult) {
        self.preferred_model = config.preferred_model.as_ref().map(|model| DisplayValue {
            full: format!("{}/{}", model.provider, model.model),
            compact: model.model.clone(),
        });
    }

    pub(crate) fn text_for_width(&self, width: usize) -> String {
        if width == 0 {
            return String::new();
        }

        let mut candidates = Vec::new();
        if let Some(model) = &self.preferred_model {
            push_candidate(
                &mut candidates,
                format!("{}{SEPARATOR}{}", model.full, self.workspace.full),
            );
            push_candidate(
                &mut candidates,
                format!("{}{SEPARATOR}{}", model.full, self.workspace.compact),
            );
            push_candidate(
                &mut candidates,
                format!("{}{SEPARATOR}{}", model.compact, self.workspace.compact),
            );
            push_candidate(&mut candidates, model.full.clone());
            push_candidate(&mut candidates, model.compact.clone());
        } else {
            push_candidate(&mut candidates, self.workspace.full.clone());
            push_candidate(&mut candidates, self.workspace.compact.clone());
        }

        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.width() <= width)
        {
            return candidate.clone();
        }

        let fallback = self
            .preferred_model
            .as_ref()
            .map(|model| model.compact.as_str())
            .unwrap_or(self.workspace.compact.as_str());
        truncate_with_ellipsis(fallback, width)
    }
}

fn push_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    if width == 1 {
        return "…".into();
    }

    let content_width = width - 1;
    let mut rendered = String::new();
    let mut rendered_width = 0;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if rendered_width + character_width > content_width {
            break;
        }
        rendered.push(character);
        rendered_width += character_width;
    }
    rendered.push('…');
    rendered
}

#[cfg(test)]
#[path = "status_line_tests.rs"]
mod tests;
