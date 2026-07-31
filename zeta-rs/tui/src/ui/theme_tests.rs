use ratatui::style::Color;
use zeta_theme::{ColorScheme, ThemeCatalog};

use super::{TerminalColorCapability, TuiTheme};

#[test]
fn tui_projects_only_its_theme_subset_for_each_terminal_capability() {
    let snapshot = ThemeCatalog::embedded()
        .unwrap()
        .built_in(ColorScheme::Dark)
        .unwrap();

    let true_color =
        TuiTheme::from_snapshot(&snapshot, TerminalColorCapability::TrueColor).unwrap();
    let ansi256 = TuiTheme::from_snapshot(&snapshot, TerminalColorCapability::Ansi256).unwrap();
    let ansi16 = TuiTheme::from_snapshot(&snapshot, TerminalColorCapability::Ansi16).unwrap();
    let monochrome =
        TuiTheme::from_snapshot(&snapshot, TerminalColorCapability::Monochrome).unwrap();

    assert!(matches!(true_color.accent, Color::Rgb(..)));
    assert!(matches!(ansi256.accent, Color::Indexed(..)));
    assert!(!matches!(
        ansi16.accent,
        Color::Rgb(..) | Color::Indexed(..)
    ));
    assert_eq!(monochrome.accent, Color::Reset);
}
