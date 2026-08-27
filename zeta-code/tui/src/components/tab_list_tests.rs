use super::TabListInputOutcome;
use super::TabListItem;
use super::TabListState;
use super::desired_height;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

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
fn horizontal_navigation_switches_tabs_and_wraps() {
    let mut tabs = TabListState::new(vec![TestTab("Overview"), TestTab("Providers")]);

    assert_eq!(
        tabs.handle_key(key(KeyCode::Right)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Providers");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Tab)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Overview");
    assert_eq!(
        tabs.handle_key(key(KeyCode::Left)),
        TabListInputOutcome::ActiveChanged
    );
    assert_eq!(tabs.active_tab().tab_label(), "Providers");
}

#[test]
fn replacing_tabs_preserves_and_clamps_the_active_index() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two"), TestTab("Three")]);
    tabs.handle_key(key(KeyCode::Left));

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
fn draw_highlights_the_active_tab() {
    let mut tabs = TabListState::new(vec![TestTab("One"), TestTab("Two")]);
    tabs.handle_key(key(KeyCode::Right));
    let backend = TestBackend::new(20, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| super::draw(frame, frame.area(), &tabs, Color::Blue))
        .unwrap();

    let active_label = &terminal.backend().buffer()[(8, 0)];
    assert_eq!(active_label.symbol(), "T");
    assert_eq!(active_label.bg, Color::Blue);
}
