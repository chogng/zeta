use std::collections::BTreeMap;

use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionActivationMode;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionPreview;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::features::theme::ThemePickerCatalog;
use crate::features::theme::ThemePickerChoice;
use crate::features::theme::ThemePickerTarget;
use crate::features::theme::ThemePreviewPalette;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ThemeSelectionAction {
    Select { preference: String },
    SelectCustom { preference: String },
    OpenCustomThemes,
}

#[derive(Clone, Copy)]
enum ThemePickerLevel {
    Main,
    Custom,
}

pub(crate) struct ThemeSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ThemeSelectionAction>,
}

pub(crate) fn theme_selection_view(catalog: &ThemePickerCatalog) -> ThemeSelectionView {
    selection_view("Theme", &catalog.choices, ThemePickerLevel::Main)
}

pub(crate) fn custom_theme_selection_view(catalog: &ThemePickerCatalog) -> ThemeSelectionView {
    selection_view(
        "Custom color themes",
        &catalog.custom_choices,
        ThemePickerLevel::Custom,
    )
}

fn selection_view(
    title: &str,
    choices: &[ThemePickerChoice],
    level: ThemePickerLevel,
) -> ThemeSelectionView {
    let mut actions = BTreeMap::new();
    let mut selected = 0;
    let items = if choices.is_empty() {
        vec![SelectionItem::new("No custom color themes found")]
    } else {
        choices
            .iter()
            .enumerate()
            .map(|(index, choice)| {
                if choice.selected {
                    selected = index;
                }
                theme_item(index, choice, level, &mut actions)
            })
            .collect()
    };
    ThemeSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(title, vec![SelectionTab::new("Themes", items)])
                .with_activation_mode(SelectionActivationMode::Enter)
                .without_tab_bar()
                .with_title_top_margin(1)
                .with_title_bottom_margin(1)
                .with_initial_selected(selected)
                .with_empty_message("No color themes available"),
            "↑/↓ select  ·  Enter apply  ·  Esc back",
        ),
        actions,
    }
}

fn theme_item(
    index: usize,
    choice: &ThemePickerChoice,
    level: ThemePickerLevel,
    actions: &mut BTreeMap<SelectionItemId, ThemeSelectionAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(format!("theme-{index}"));
    let action = match &choice.target {
        ThemePickerTarget::Preference(preference) => match level {
            ThemePickerLevel::Main => ThemeSelectionAction::Select {
                preference: preference.clone(),
            },
            ThemePickerLevel::Custom => ThemeSelectionAction::SelectCustom {
                preference: preference.clone(),
            },
        },
        ThemePickerTarget::CustomThemes => ThemeSelectionAction::OpenCustomThemes,
    };
    actions.insert(item_id.clone(), action);
    let current = if choice.selected { " ✓" } else { "" };
    SelectionItem::new(format!("{}. {}{current}", index + 1, choice.label))
        .with_id(item_id)
        .with_selection_foreground(choice.palette.highlight)
        .with_preview(
            SelectionPreview::new("Diff preview", diff_preview(choice.palette))
                .with_caption(Line::from(Span::styled(
                    format!("Syntax palette: {}", choice.palette_label),
                    Style::default().fg(choice.palette.muted),
                )))
                .with_separator_color(choice.palette.muted)
                .with_margins(2, 0),
        )
}

fn diff_preview(palette: ThemePreviewPalette) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled("1   ", Style::default().fg(palette.muted)),
            Span::styled("fn", Style::default().fg(palette.keyword).bold()),
            Span::raw(" "),
            Span::styled("greet", Style::default().fg(palette.function)),
            Span::styled("(zeta: &", Style::default().fg(palette.foreground)),
            Span::styled("str", Style::default().fg(palette.r#type)),
            Span::styled(") -> ", Style::default().fg(palette.foreground)),
            Span::styled("String", Style::default().fg(palette.r#type)),
            Span::styled(" {", Style::default().fg(palette.foreground)),
        ])
        .style(
            Style::default()
                .fg(palette.foreground)
                .bg(palette.background),
        ),
        Line::from(vec![
            Span::styled("2  -  ", Style::default().fg(palette.removed_marker)),
            Span::styled("  format!", Style::default().fg(palette.function)),
            Span::styled("(", Style::default().fg(palette.foreground)),
            Span::styled("\"Hello, {}!\"", Style::default().fg(palette.string)),
            Span::styled(", ", Style::default().fg(palette.foreground)),
            Span::styled("zeta", Style::default().fg(palette.variable)),
            Span::styled(")", Style::default().fg(palette.foreground)),
        ])
        .style(
            Style::default()
                .fg(palette.foreground)
                .bg(palette.removed_background),
        ),
        Line::from(vec![
            Span::styled("2  +  ", Style::default().fg(palette.inserted_marker)),
            Span::styled("  format!", Style::default().fg(palette.function)),
            Span::styled("(", Style::default().fg(palette.foreground)),
            Span::styled("\"Hello, {zeta}!\"", Style::default().fg(palette.string)),
            Span::styled(")", Style::default().fg(palette.foreground)),
        ])
        .style(
            Style::default()
                .fg(palette.foreground)
                .bg(palette.inserted_background),
        ),
        Line::from(vec![
            Span::styled("3   ", Style::default().fg(palette.muted)),
            Span::styled("}", Style::default().fg(palette.foreground)),
        ])
        .style(
            Style::default()
                .fg(palette.foreground)
                .bg(palette.background),
        ),
    ]
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
