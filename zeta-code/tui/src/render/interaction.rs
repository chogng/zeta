use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;

use super::RenderContext;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum InteractionTarget {
    #[default]
    Rest,
    Active,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum InteractionAttention {
    #[default]
    None,
    Keyboard,
    Hovered,
    Pressed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct InteractionState {
    pub(crate) target: InteractionTarget,
    pub(crate) attention: InteractionAttention,
}

pub(crate) fn interaction_style(context: RenderContext<'_>, state: InteractionState) -> Style {
    if state.target == InteractionTarget::Disabled {
        return Style::default()
            .fg(context.disabled_foreground())
            .add_modifier(Modifier::DIM);
    }

    let mut style = match state.attention {
        InteractionAttention::None => match state.target {
            InteractionTarget::Active => Style::default()
                .fg(context.accent_surface_foreground())
                .bg(context.accent_surface_background()),
            InteractionTarget::Rest | InteractionTarget::Disabled => Style::default(),
        },
        InteractionAttention::Keyboard => Style::default()
            .fg(context.selection_foreground())
            .bg(context.selection_background()),
        InteractionAttention::Hovered => Style::default()
            .fg(context.hover_foreground())
            .bg(context.hover_background()),
        InteractionAttention::Pressed => Style::default()
            .fg(context.pressed_foreground())
            .bg(context.pressed_background()),
    };
    if state.target == InteractionTarget::Active
        || state.attention == InteractionAttention::Keyboard
        || state.attention == InteractionAttention::Pressed
    {
        style = style.add_modifier(Modifier::BOLD);
    }
    if interaction_colors_are_unavailable(context) {
        style = match state.attention {
            InteractionAttention::Hovered => style.add_modifier(Modifier::UNDERLINED),
            InteractionAttention::Keyboard | InteractionAttention::Pressed => {
                style.add_modifier(Modifier::REVERSED)
            }
            InteractionAttention::None if state.target == InteractionTarget::Active => {
                style.add_modifier(Modifier::REVERSED)
            }
            InteractionAttention::None => style,
        };
    }
    style
}

pub(crate) fn focus_style(context: RenderContext<'_>) -> Style {
    Style::default()
        .fg(context.focus())
        .add_modifier(Modifier::BOLD)
}

fn interaction_colors_are_unavailable(context: RenderContext<'_>) -> bool {
    context.focus() == Color::Reset
        && context.selection_background() == Color::Reset
        && context.hover_background() == Color::Reset
}

#[cfg(test)]
#[path = "interaction_tests.rs"]
mod tests;
