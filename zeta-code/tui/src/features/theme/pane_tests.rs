use std::collections::BTreeMap;

use super::ThemeSelectionAction;
use super::custom_theme_selection_view;
use super::theme_selection_view;
use crate::components::selection::SelectionInputOutcome;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionViewState;
use crate::features::theme::ThemePickerCatalog;
use crate::features::theme::ThemePickerChoice;
use crate::features::theme::ThemePickerTarget;
use crate::features::theme::ThemePreviewPalette;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

fn palette(highlight: Color) -> ThemePreviewPalette {
    ThemePreviewPalette {
        background: Color::Black,
        border: Color::Gray,
        foreground: Color::White,
        muted: Color::DarkGray,
        highlight,
        keyword: Color::Red,
        string: Color::Blue,
        function: Color::Magenta,
        r#type: Color::Cyan,
        variable: Color::Yellow,
        inserted_background: Color::Green,
        removed_background: Color::Red,
        inserted_marker: Color::LightGreen,
        removed_marker: Color::LightRed,
    }
}

fn catalog() -> ThemePickerCatalog {
    let labels = [
        "Auto",
        "Dark mode",
        "Light mode",
        "Dark mode (colorblind-friendly)",
        "Light mode (colorblind-friendly)",
        "Dark mode (ANSI colors only)",
        "Light mode (ANSI colors only)",
    ];
    let mut choices = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| ThemePickerChoice {
            label: label.into(),
            palette_label: format!("Palette {index}"),
            target: ThemePickerTarget::Preference(format!("theme-{index}")),
            palette: palette(Color::Indexed(index as u8)),
            selected: index == 1,
        })
        .collect::<Vec<_>>();
    choices.push(ThemePickerChoice {
        label: "Custom color theme".into(),
        palette_label: "User-defined".into(),
        target: ThemePickerTarget::CustomThemes,
        palette: palette(Color::Magenta),
        selected: false,
    });
    ThemePickerCatalog {
        choices,
        custom_choices: vec![ThemePickerChoice {
            label: "Aurora".into(),
            palette_label: "User-defined · Aurora".into(),
            target: ThemePickerTarget::Preference("aurora".into()),
            palette: palette(Color::Cyan),
            selected: false,
        }],
    }
}

#[test]
fn theme_pane_is_numbered_fixed_and_not_searchable() {
    let view = theme_selection_view(&catalog());
    let (model, key_hints) = view.model.into_parts();
    let mut state = SelectionViewState::new(model);

    assert_eq!(state.title(), "Theme");
    assert!(!state.show_tabs());
    assert_eq!(state.visible_items().len(), 8);
    assert_eq!(state.visible_items()[0].label(), "1. Auto");
    assert_eq!(state.visible_items()[1].label(), "2. Dark mode ✓");
    assert_eq!(state.visible_items()[7].label(), "8. Custom color theme");
    assert_eq!(state.selected_visible_index(), Some(1));
    assert_eq!(
        state.selected_item().unwrap().selection_foreground(),
        Some(Color::Indexed(1))
    );
    let preview = state.selected_item().unwrap().preview().unwrap();
    assert_eq!(preview.title(), "Diff preview");
    assert_eq!(preview.lines().len(), 4);
    let preview_text = preview
        .lines()
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref() as &str)
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert!(preview_text[0].starts_with("1   fn greet(zeta:"));
    assert!(preview_text[1].starts_with("2  -"));
    assert!(preview_text[2].starts_with("2  +"));
    assert!(preview_text[3].starts_with("3"));
    let caption = preview
        .caption()
        .unwrap()
        .spans
        .iter()
        .map(|span| span.content.as_ref() as &str)
        .collect::<String>();
    assert_eq!(caption, "Syntax palette: Palette 1");
    let height = crate::components::pane::desired_height(state.desired_height(80));
    let backend = TestBackend::new(80, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let areas = crate::components::pane::areas(frame.area());
            crate::components::selection::draw(frame, areas.body, &state);
            crate::components::key_hint_bar::draw(frame, areas.key_hint_bar, &key_hints);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rows = (0..height)
        .map(|row| {
            (0..80)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let rendered = rows.join("\n");
    let title_row = rows.iter().position(|row| row.contains("Theme")).unwrap();
    let first_choice_row = rows.iter().position(|row| row.contains("1. Auto")).unwrap();
    assert!(rendered.contains("Diff preview"));
    assert!(rendered.contains("Syntax palette: Palette 1"));
    assert!(rendered.contains('╌'));
    assert!(!rendered.contains('┌'));
    assert!(!rendered.contains('┘'));
    assert_eq!(title_row, 2);
    assert_eq!(first_choice_row - title_row, 2);
    let custom_row = rows
        .iter()
        .position(|row| row.contains("8. Custom color theme"))
        .unwrap();
    let preview_row = rows
        .iter()
        .position(|row| row.contains("Diff preview"))
        .unwrap();
    let palette_row = rows
        .iter()
        .position(|row| row.contains("Syntax palette"))
        .unwrap();
    let key_hint_row = rows
        .iter()
        .position(|row| row.contains("Enter apply"))
        .unwrap();
    assert_eq!(preview_row - custom_row, 3);
    assert_eq!(key_hint_row - palette_row, 3);
    assert_eq!(buffer[(2, preview_row as u16)].fg, Color::DarkGray);

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        SelectionInputOutcome::Consumed
    );
    assert!(!state.search_active());
    assert_eq!(state.query(), "");
}

#[test]
fn custom_row_opens_the_custom_theme_model() {
    let catalog = catalog();
    let view = theme_selection_view(&catalog);
    assert_eq!(
        view.actions.get(&SelectionItemId::new("theme-7")),
        Some(&ThemeSelectionAction::OpenCustomThemes)
    );

    let custom = custom_theme_selection_view(&catalog);
    let state = SelectionViewState::new(custom.model.into_body());
    assert_eq!(state.title(), "Custom color themes");
    assert_eq!(state.visible_items()[0].label(), "1. Aurora");
    assert_eq!(
        custom.actions,
        BTreeMap::from([(
            SelectionItemId::new("theme-0"),
            ThemeSelectionAction::SelectCustom {
                preference: "aurora".into(),
            },
        )])
    );
}
