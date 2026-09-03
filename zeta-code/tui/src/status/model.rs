use crate::thread::TurnApprovalModes;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitHeadDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_protocol::ApprovalMode;
use zeta_protocol::ModelMoneyAmount;
use zeta_protocol::ModelReferenceCostSummary;
use zeta_protocol::ModelUsageSummary;

use super::StatusLineItem;
use super::StatusLineSettings;

const SEPARATOR: &str = " · ";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct StatusLineRuntime {
    pub(crate) plan: Option<(usize, usize)>,
    pub(crate) queue: usize,
    pub(crate) subagents: usize,
}

impl StatusLineRuntime {
    pub(crate) fn text(self) -> String {
        let mut segments = Vec::new();
        if let Some((completed, total)) = self.plan {
            segments.push(format!("plan {completed}/{total}"));
        }
        for (label, count) in [("queue", self.queue), ("subagents", self.subagents)] {
            if count > 0 {
                segments.push(format!("{label} {count}"));
            }
        }
        segments.join(SEPARATOR)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ApprovalModeDisplay {
    pub(super) icon: &'static str,
    pub(super) label: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DisplayValue {
    full: String,
    compact: String,
}

/// Pure display model for the configured context rendered inside StatusLine.
///
/// Data acquisition remains with the application and the typed interfaces that own each value.
/// This model only keeps display variants and selects the richest configured representation that
/// fits.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StatusLineModel {
    settings: StatusLineSettings,
    preferred_model: Option<DisplayValue>,
    cache_hit_rate: Option<DisplayValue>,
    reference_cost: Option<DisplayValue>,
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
            full: model.model.clone(),
            compact: model.model.clone(),
        });
    }

    pub(crate) fn apply_thread_accounting(
        &mut self,
        usage: &ModelUsageSummary,
        reference_cost: &ModelReferenceCostSummary,
    ) {
        self.cache_hit_rate = cache_hit_rate_display(usage);
        self.reference_cost = reference_cost_display(usage.model_invocations, reference_cost);
    }

    pub(crate) fn clear_thread_accounting(&mut self) {
        self.cache_hit_rate = None;
        self.reference_cost = None;
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

    pub(crate) fn top_text_for_width(&self, width: usize, runtime: StatusLineRuntime) -> String {
        let runtime = runtime.text();
        let mut values = Vec::new();
        if !runtime.is_empty() {
            values.push(DisplayValue {
                full: runtime.clone(),
                compact: runtime,
            });
        }
        values.extend(self.configured_values());
        fit_values(&values, width)
    }

    pub(crate) fn policy_text_for_width(
        &self,
        width: usize,
        approval: impl Into<TurnApprovalModes>,
    ) -> String {
        if !self.settings.enabled(StatusLineItem::Permissions) {
            return String::new();
        }
        truncate_with_ellipsis(&approval_mode_text(approval.into()), width)
    }

    fn configured_values(&self) -> Vec<DisplayValue> {
        let mut values = Vec::new();
        for item in self.settings.items() {
            match item {
                StatusLineItem::Permissions => {}
                StatusLineItem::Model => values.extend(self.preferred_model.iter().cloned()),
                StatusLineItem::CacheHitRate => values.extend(self.cache_hit_rate.iter().cloned()),
                StatusLineItem::ReferenceCost => values.extend(self.reference_cost.iter().cloned()),
                StatusLineItem::GitBranch => values.extend(self.git_branch.iter().cloned()),
                StatusLineItem::GitChanges => values.extend(self.git_changes.iter().cloned()),
            }
        }
        values
    }
}

fn cache_hit_rate_display(usage: &ModelUsageSummary) -> Option<DisplayValue> {
    format_cache_hit_rate(usage).map(|percentage| DisplayValue {
        full: format!("cache hit {percentage}"),
        compact: if percentage == "unknown" {
            "cache ?".into()
        } else {
            format!("cache {percentage}")
        },
    })
}

fn reference_cost_display(
    model_invocations: u64,
    summary: &ModelReferenceCostSummary,
) -> Option<DisplayValue> {
    format_reference_cost(model_invocations, summary).map(|amount| DisplayValue {
        full: format!("cost {amount}"),
        compact: if amount == "unknown" {
            "cost ?".into()
        } else {
            amount
        },
    })
}

pub(super) fn format_cache_hit_rate(usage: &ModelUsageSummary) -> Option<String> {
    if usage.model_invocations == 0 {
        return None;
    }
    let input = &usage.input_tokens;
    let cached = &usage.cached_input_tokens;
    if !input.complete
        || !cached.complete
        || input.reported == 0
        || cached.reported > input.reported
    {
        return Some("unknown".into());
    }
    let percentage_tenths = u128::from(cached.reported) * 1_000 / u128::from(input.reported);
    Some(format!(
        "{}.{:01}%",
        percentage_tenths / 10,
        percentage_tenths % 10
    ))
}

pub(super) fn format_reference_cost(
    model_invocations: u64,
    summary: &ModelReferenceCostSummary,
) -> Option<String> {
    if model_invocations == 0 {
        return None;
    }
    let [amount] = summary.known_amounts.as_slice() else {
        return Some("unknown".into());
    };
    let Some(amount) = format_money(amount) else {
        return Some("unknown".into());
    };
    let prefix = if summary.complete { "" } else { "≥" };
    Some(format!("{prefix}{amount}"))
}

fn format_money(amount: &ModelMoneyAmount) -> Option<String> {
    let pico_units = amount.pico_units.parse::<u128>().ok()?;
    let whole = pico_units / 1_000_000_000_000;
    let remainder = pico_units % 1_000_000_000_000;
    let number = if remainder == 0 {
        whole.to_string()
    } else {
        let fraction = format!("{remainder:012}");
        format!("{whole}.{}", fraction.trim_end_matches('0'))
    };
    Some(if amount.currency == "USD" {
        format!("${number}")
    } else {
        format!("{} {number}", amount.currency)
    })
}

fn fit_values(values: &[DisplayValue], width: usize) -> String {
    if width == 0 {
        return String::new();
    }

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

pub(super) fn approval_mode_text(approval: TurnApprovalModes) -> String {
    let next = approval_mode_display(approval.next);
    match approval.current {
        Some(current) if current != approval.next => {
            let current = approval_mode_display(current);
            format!(
                "{} current: {} · {} next: {}",
                current.icon, current.label, next.icon, next.label
            )
        }
        _ => format!("{} {}", next.icon, next.label),
    }
}

pub(super) fn approval_mode_display(approval_mode: ApprovalMode) -> ApprovalModeDisplay {
    match approval_mode {
        ApprovalMode::AskPermissions => ApprovalModeDisplay {
            icon: "⏸",
            label: "ask permissions on",
        },
        ApprovalMode::AutoReview => ApprovalModeDisplay {
            icon: "⏩",
            label: "auto review on",
        },
        ApprovalMode::BypassPermissions => ApprovalModeDisplay {
            icon: "▶",
            label: "bypass permissions on",
        },
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
