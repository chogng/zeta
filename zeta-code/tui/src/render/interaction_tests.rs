use super::InteractionAttention;
use super::InteractionState;
use super::InteractionTarget;
use super::interaction_style;
use crate::render::RenderContext;
use crate::render::RenderTheme;
use crate::render::ThemePalette;
use crate::render::test_context;
use ratatui::style::Color;
use ratatui::style::Modifier;
use zeta_terminal_detection::ColorLevel;

#[test]
fn interaction_states_use_distinct_theme_pairs() {
    let context = test_context();

    let selected = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention: InteractionAttention::Keyboard,
        },
    );
    let hovered = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention: InteractionAttention::Hovered,
        },
    );
    let pressed = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention: InteractionAttention::Pressed,
        },
    );
    let disabled = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Disabled,
            attention: InteractionAttention::Hovered,
        },
    );

    assert_eq!(selected.fg, Some(context.selection_foreground()));
    assert_eq!(selected.bg, Some(context.selection_background()));
    assert_eq!(hovered.fg, Some(context.hover_foreground()));
    assert_eq!(hovered.bg, Some(context.hover_background()));
    assert_eq!(pressed.fg, Some(context.pressed_foreground()));
    assert_eq!(pressed.bg, Some(context.pressed_background()));
    assert_eq!(disabled.fg, Some(context.disabled_foreground()));
    assert!(disabled.add_modifier.contains(Modifier::DIM));
}

#[test]
fn monochrome_preserves_interaction_without_color() {
    let theme = RenderTheme::from_palette(ThemePalette::dark(), ColorLevel::Monochrome);
    let context = RenderContext::new(&theme, 0);

    let hovered = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention: InteractionAttention::Hovered,
        },
    );
    let selected = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            attention: InteractionAttention::Keyboard,
        },
    );

    assert_eq!(hovered.fg, Some(Color::Reset));
    assert!(hovered.add_modifier.contains(Modifier::UNDERLINED));
    assert!(selected.add_modifier.contains(Modifier::REVERSED));
}
