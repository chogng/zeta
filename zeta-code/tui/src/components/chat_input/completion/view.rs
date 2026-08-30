mod mention {
    use super::super::MentionPopupView;
    use super::super::mention::MentionMatchKind;
    use crate::render::RenderContext;
    use crate::render::bottom_anchored_area;
    use crate::render::horizontal_margin;
    use ratatui::Frame;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;
    use ratatui::style::Style;
    use ratatui::text::Line;
    use ratatui::text::Span;
    use ratatui::widgets::Clear;
    use ratatui::widgets::Paragraph;

    #[derive(Clone, Copy)]
    struct PopupLayout {
        area: Rect,
        first_path: usize,
        visible_rows: usize,
    }

    pub(crate) fn draw(
        frame: &mut Frame<'_>,
        area: Rect,
        popup: Option<MentionPopupView<'_>>,
        hovered: Option<usize>,
        pressed: Option<usize>,
        context: RenderContext<'_>,
    ) {
        let Some(popup) = popup else {
            return;
        };
        let Some(layout) = popup_layout(area, popup.matches.len(), popup.selected) else {
            return;
        };
        let lines = if popup.matches.is_empty() {
            vec![Line::from(Span::styled(
                if popup.searching {
                    "Searching files and plugins…"
                } else {
                    "No matching files or plugins"
                },
                Style::default().fg(context.muted()),
            ))]
        } else {
            popup
                .matches
                .iter()
                .enumerate()
                .skip(layout.first_path)
                .take(layout.visible_rows)
                .map(|(index, mention_match)| {
                    let base_style =
                        super::item_style(index, popup.selected, hovered, pressed, context);
                    let mut matched_indices = mention_match.indices.iter().peekable();
                    let mut spans = mention_match
                        .label
                        .chars()
                        .enumerate()
                        .map(|(char_index, character)| {
                            let mut style = base_style;
                            if matched_indices
                                .peek()
                                .is_some_and(|matched| **matched == char_index)
                            {
                                matched_indices.next();
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            Span::styled(character.to_string(), style)
                        })
                        .collect::<Vec<_>>();
                    if mention_match.kind == MentionMatchKind::Plugin {
                        spans.push(Span::styled("  plugin", base_style));
                    }
                    Line::from(spans)
                })
                .collect()
        };
        frame.render_widget(Clear, layout.area);
        frame.render_widget(Paragraph::new(lines), layout.area);
    }

    pub(crate) fn mention_index_at(
        area: Rect,
        popup: Option<MentionPopupView<'_>>,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        let popup = popup?;
        if popup.matches.is_empty() {
            return None;
        }
        let layout = popup_layout(area, popup.matches.len(), popup.selected)?;
        if column < layout.area.x
            || column >= layout.area.right()
            || row < layout.area.y
            || row >= layout.area.bottom()
        {
            return None;
        }
        let index = layout.first_path + usize::from(row - layout.area.y);
        (index < popup.matches.len()).then_some(index)
    }

    fn popup_layout(area: Rect, path_count: usize, selected: usize) -> Option<PopupLayout> {
        let max_rows = area.height.saturating_sub(2).min(6) as usize;
        if max_rows == 0 {
            return None;
        }
        let visible_rows = path_count.clamp(1, max_rows);
        let first_path = selected
            .saturating_add(1)
            .saturating_sub(max_rows)
            .min(path_count.saturating_sub(visible_rows));
        Some(PopupLayout {
            area: horizontal_margin(bottom_anchored_area(area, visible_rows as u16), 2),
            first_path,
            visible_rows,
        })
    }
}

mod skill {
    use super::super::SkillCompletionView;
    use crate::render::RenderContext;
    use crate::render::bottom_anchored_area;
    use crate::render::horizontal_margin;
    use ratatui::Frame;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::text::Line;
    use ratatui::text::Span;
    use ratatui::widgets::Clear;
    use ratatui::widgets::Paragraph;

    const MAX_VISIBLE_ROWS: usize = 6;

    pub(crate) fn draw(
        frame: &mut Frame<'_>,
        area: Rect,
        popup: Option<SkillCompletionView<'_>>,
        hovered: Option<usize>,
        pressed: Option<usize>,
        context: RenderContext<'_>,
    ) {
        let Some(popup) = popup else {
            return;
        };
        let Some((popup_area, first_item, visible_rows)) = popup_layout(area, popup) else {
            return;
        };
        let lines = if popup.items.is_empty() {
            vec![Line::from(Span::styled(
                "No matching Skills",
                Style::default().fg(context.muted()),
            ))]
        } else {
            popup
                .items
                .iter()
                .enumerate()
                .skip(first_item)
                .take(visible_rows)
                .map(|(index, item)| {
                    let style = super::item_style(index, popup.selected, hovered, pressed, context);
                    Line::from(vec![
                        Span::styled(format!("${}", item.name()), style),
                        Span::styled(format!("  {}", item.description()), style),
                    ])
                })
                .collect()
        };
        frame.render_widget(Clear, popup_area);
        frame.render_widget(Paragraph::new(lines), popup_area);
    }

    pub(crate) fn skill_index_at(
        area: Rect,
        popup: Option<SkillCompletionView<'_>>,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        let popup = popup?;
        if popup.items.is_empty() {
            return None;
        }
        let (popup_area, first_item, _) = popup_layout(area, popup)?;
        if column < popup_area.x
            || column >= popup_area.right()
            || row < popup_area.y
            || row >= popup_area.bottom()
        {
            return None;
        }
        let index = first_item + usize::from(row - popup_area.y);
        (index < popup.items.len()).then_some(index)
    }

    fn popup_layout(area: Rect, popup: SkillCompletionView<'_>) -> Option<(Rect, usize, usize)> {
        let max_rows = area.height.saturating_sub(2).min(MAX_VISIBLE_ROWS as u16) as usize;
        if max_rows == 0 {
            return None;
        }
        let visible_rows = popup.items.len().clamp(1, max_rows);
        let first_item = popup
            .selected
            .saturating_add(1)
            .saturating_sub(max_rows)
            .min(popup.items.len().saturating_sub(visible_rows));
        Some((
            horizontal_margin(bottom_anchored_area(area, visible_rows as u16), 2),
            first_item,
            visible_rows,
        ))
    }
}

mod slash {
    use super::super::SlashCommandsView;
    use crate::render::RenderContext;
    use crate::render::bottom_anchored_area;
    use crate::render::horizontal_margin;
    use ratatui::Frame;
    use ratatui::layout::Rect;
    use ratatui::style::Style;
    use ratatui::text::Line;
    use ratatui::text::Span;
    use ratatui::widgets::Paragraph;

    const COMMAND_COLUMN_WIDTH: usize = 26;

    #[derive(Clone, Copy)]
    struct PopupLayout {
        area: Rect,
        first_command: usize,
        visible_rows: usize,
    }

    pub(crate) fn draw(
        frame: &mut Frame<'_>,
        area: Rect,
        popup: Option<SlashCommandsView<'_>>,
        hovered: Option<usize>,
        pressed: Option<usize>,
        context: RenderContext<'_>,
    ) {
        let Some(popup) = popup else {
            return;
        };
        let Some(layout) = popup_layout(area, popup.commands.len(), popup.selected) else {
            return;
        };
        let lines = if popup.commands.is_empty() {
            vec![Line::from(Span::styled(
                "No matching commands",
                Style::default().fg(context.muted()),
            ))]
        } else {
            popup
                .commands
                .iter()
                .enumerate()
                .skip(layout.first_command)
                .take(layout.visible_rows)
                .map(|(index, command)| {
                    let command_style =
                        super::item_style(index, popup.selected, hovered, pressed, context);
                    Line::from(vec![
                        Span::styled(
                            format!("/{:<width$}", command.name, width = COMMAND_COLUMN_WIDTH),
                            command_style,
                        ),
                        Span::styled(&command.description, command_style),
                    ])
                })
                .collect()
        };
        frame.render_widget(Paragraph::new(lines), layout.area);
    }

    pub(crate) fn command_index_at(
        area: Rect,
        popup: Option<SlashCommandsView<'_>>,
        column: u16,
        row: u16,
    ) -> Option<usize> {
        let popup = popup?;
        if popup.commands.is_empty() {
            return None;
        }
        let layout = popup_layout(area, popup.commands.len(), popup.selected)?;
        if column < layout.area.x
            || column >= layout.area.right()
            || row < layout.area.y
            || row >= layout.area.bottom()
        {
            return None;
        }
        let index = layout.first_command + usize::from(row - layout.area.y);
        (index < popup.commands.len()).then_some(index)
    }

    fn popup_layout(area: Rect, command_count: usize, selected: usize) -> Option<PopupLayout> {
        let max_rows = area.height.saturating_sub(2).min(6) as usize;
        if max_rows == 0 {
            return None;
        }
        let visible_rows = command_count.clamp(1, max_rows);
        let first_command = selected
            .saturating_add(1)
            .saturating_sub(max_rows)
            .min(command_count.saturating_sub(visible_rows));
        Some(PopupLayout {
            area: horizontal_margin(bottom_anchored_area(area, visible_rows as u16), 2),
            first_command,
            visible_rows,
        })
    }
}

use super::CompletionView;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    completion: Option<CompletionView<'_>>,
    hovered: Option<usize>,
    pressed: Option<usize>,
    context: RenderContext<'_>,
) {
    match completion {
        Some(CompletionView::Slash(view)) => {
            slash::draw(frame, area, Some(view), hovered, pressed, context)
        }
        Some(CompletionView::Mention(view)) => {
            mention::draw(frame, area, Some(view), hovered, pressed, context)
        }
        Some(CompletionView::Skill(view)) => {
            skill::draw(frame, area, Some(view), hovered, pressed, context)
        }
        None => {}
    }
}

fn item_style(
    index: usize,
    selected: usize,
    hovered: Option<usize>,
    pressed: Option<usize>,
    context: RenderContext<'_>,
) -> ratatui::style::Style {
    let state = InteractionState {
        target: InteractionTarget::Rest,
        selected: index == selected,
        hovered: hovered == Some(index),
        pressed: pressed == Some(index),
    };
    let style = interaction_style(context, state);
    if !state.selected && !state.hovered && !state.pressed {
        style.fg(context.muted())
    } else {
        style
    }
}

pub(crate) fn index_at(
    area: Rect,
    completion: Option<CompletionView<'_>>,
    column: u16,
    row: u16,
) -> Option<usize> {
    match completion {
        Some(CompletionView::Slash(view)) => slash::command_index_at(area, Some(view), column, row),
        Some(CompletionView::Mention(view)) => {
            mention::mention_index_at(area, Some(view), column, row)
        }
        Some(CompletionView::Skill(view)) => skill::skill_index_at(area, Some(view), column, row),
        None => None,
    }
}
