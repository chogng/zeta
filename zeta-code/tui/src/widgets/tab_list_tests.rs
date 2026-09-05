use super::TabListInputOutcome;
use super::TabListItem;
use super::TabListState;
use super::desired_height;
use crate::render::test_context;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TestTab(&'static str);

impl TabListItem for TestTab {
    fn tab_label(&self) -> &str {
        self.0
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn keyboard_navigation_switches_tabs_in_both_directions_and_wraps() {
    let mut tabs = TabListState::new(vec![TestTab("Overview"), TestTab("Providers")]);

    assert_eq!(
        tabs.handle_key(key(KeyCode::Right)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Providers");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Left)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Overview");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Tab)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Providers");
    assert_eq!(
        tabs.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Overview");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Left)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Providers");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Right)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Overview");
}

#[test]
fn mouse_hit_testing_selects_tabs_and_ignores_the_gap() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two")]);
    let area = Rect::new(4, 7, 20, 1);

    assert_eq!(tabs.index_at(area, 4, 7), Some(0));
    assert_eq!(tabs.index_at(area, 9, 7), None);
    let second = tabs.index_at(area, 11, 7).unwrap();
    assert_eq!(tabs.select(second), TabListInputOutcome::ActiveChanged);
    assert_eq!(tabs.active_index(), 1);
    assert_eq!(tabs.select(second), TabListInputOutcome::Consumed);
}

#[test]
fn mouse_hit_testing_follows_wrapped_tabs() {
    let tabs = TabListState::new(vec![TestTab("One"), TestTab("Two")]);
    let area = Rect::new(0, 0, 10, 2);

    assert_eq!(tabs.index_at(area, 0, 1), Some(1));
}

#[test]
fn replacing_tabs_preserves_and_clamps_the_active_index() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two"), TestTab("Three")]);
    tabs.handle_key(key(KeyCode::BackTab));

    tabs.replace_tabs(vec![TestTab("First"), TestTab("Second")]);

    assert_eq!(tabs.active_index(), 1);
    assert_eq!(tabs.active_tab().tab_label(), "Second");
}

#[test]
fn narrow_width_wraps_tabs_without_hiding_them() {
    let tabs = vec![TestTab("Commands"), TestTab("Keys")];

    assert_eq!(desired_height(&tabs, 80), 1);
    assert_eq!(desired_height(&tabs, 12), 2);
}

#[test]
fn draw_keeps_the_active_tab_accent_surface_while_hovered() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two")]);
    tabs.handle_key(key(KeyCode::Tab));
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            super::draw(
                frame,
                frame.area(),
                &tabs,
                false,
                Some(1),
                None,
                test_context(),
            )
        })
        .unwrap();

    let active_label = &terminal.backend().buffer()[(8, 0)];
    assert_eq!(active_label.symbol(), "T");
    assert_eq!(active_label.fg, test_context().accent_surface_foreground());
    assert_eq!(active_label.bg, test_context().accent_surface_background());
}

#[test]
fn held_tab_does_not_skip_pages_and_shift_tab_moves_back() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two"), TestTab("Three")]);
    tabs.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
    assert_eq!(tabs.active_index(), 2);
    for kind in [
        crossterm::event::KeyEventKind::Repeat,
        crossterm::event::KeyEventKind::Release,
    ] {
        tabs.handle_key(KeyEvent::new_with_kind(
            KeyCode::Tab,
            KeyModifiers::NONE,
            kind,
        ));
    }
    assert_eq!(tabs.active_index(), 2);
}
