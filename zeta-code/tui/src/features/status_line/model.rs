use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitHeadDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::ApprovalMode;

use super::StatusLineItem;
use super::StatusLineSettings;

const SEPARATOR: &str = " · ";

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisplayValue {
    full: String,
    compact: String,
}

/// Pure display model for the configured context rendered inside the footer.
///
/// Data acquisition remains with the application and the typed interfaces that own each value.
/// This model only keeps display variants and selects the richest configured representation that
/// fits.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StatusLineModel {
    settings: StatusLineSettings,
    preferred_model: Option<DisplayValue>,
    git_branch: Option<DisplayValue>,
    git_changes: Option<DisplayValue>,
}

impl StatusLineModel {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn apply_settings(&mut self, settings: StatusLineSettings) {
        self.settings = settings;
    }

    pub(crate) fn apply_preferred_model(&mut self, model: Option<&ModelRefDto>) {
        self.preferred_model = model.map(|model| DisplayValue {
            full: format!("{}/{}", model.provider, model.model),
            compact: model.model.clone(),
        });
    }

    pub(crate) fn apply_git_status(&mut self, status: &GitStatusResult) {
        let identity = match &status.head {
            GitHeadDto::Branch { name, .. } | GitHeadDto::Unborn { name } => name.clone(),
            GitHeadDto::Detached { object_id } => {
                format!("detached@{}", object_id.chars().take(8).collect::<String>())
            }
        };
        self.git_branch = Some(DisplayValue {
            full: identity.clone(),
            compact: identity,
        });
        let change_count = status.changes.len();
        self.git_changes = (change_count > 0).then(|| DisplayValue {
            full: if change_count == 1 {
                "1 change".into()
            } else {
                format!("{change_count} changes")
            },
            compact: "*".into(),
        });
    }

    pub(crate) fn text_for_width(&self, width: usize, approval_mode: ApprovalMode) -> String {
        if width == 0 {
            return String::new();
        }

        let values = self.configured_values(approval_mode);
        if values.is_empty() {
            return String::new();
        }

        let full = values
            .iter()
            .map(|value| value.full.as_str())
            .collect::<Vec<_>>()
            .join(SEPARATOR);
        if full.width() <= width {
            return full;
        }

        let compact = values
            .iter()
            .map(|value| value.compact.as_str())
            .collect::<Vec<_>>()
            .join(SEPARATOR);
        if compact.width() <= width {
            return compact;
        }

        for visible in (1..values.len()).rev() {
            let candidate = values[..visible]
                .iter()
                .map(|value| value.compact.as_str())
                .collect::<Vec<_>>()
                .join(SEPARATOR);
            if candidate.width() <= width {
                return candidate;
            }
        }

        truncate_with_ellipsis(&values[0].compact, width)
    }

    fn configured_values(&self, approval_mode: ApprovalMode) -> Vec<DisplayValue> {
        let mut values = Vec::new();
        if self.settings.enabled(StatusLineItem::Permissions) {
            let permission = match approval_mode {
                ApprovalMode::AskPermissions => "◉ ask permissions on",
                ApprovalMode::AutoReview => "◎ auto review on",
                ApprovalMode::BypassPermissions => "⊘ bypass permissions on",
            };
            values.push(DisplayValue {
                full: permission.into(),
                compact: permission.into(),
            });
        }
        if self.settings.enabled(StatusLineItem::Model)
            && let Some(model) = &self.preferred_model
        {
            values.push(model.clone());
        }
        if self.settings.enabled(StatusLineItem::GitBranch)
            && let Some(branch) = &self.git_branch
        {
            values.push(branch.clone());
        }
        if self.settings.enabled(StatusLineItem::GitChanges)
            && let Some(changes) = &self.git_changes
        {
            values.push(changes.clone());
        }
        values
    }
}

pub(super) fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
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
#[path = "model_tests.rs"]
mod tests;
