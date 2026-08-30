use super::WelcomeModel;
use super::draw;
use crate::ui::accent;
use crate::ui::chat_input_chrome;
use crate::ui::highlight;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use std::path::Path;

#[test]
fn wide_banner_uses_the_two_column_welcome_presentation() {
    let buffer = render(80, 13);
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("/work/zeta"));
    assert!(rendered.contains("Tips for getting started"));
    assert!(rendered.contains("Use @ to mention files"));
    assert!(rendered.contains("Try asking"));
    assert_eq!(buffer[(2, 1)].fg, highlight());
    assert_eq!(buffer[(77, 11)].fg, highlight());
    assert_eq!(buffer[(30, 2)].fg, highlight());
    assert_eq!(buffer[(33, 5)].fg, highlight());
    assert_eq!(buffer[(9, 1)].symbol(), "Z");
    assert_eq!(buffer[(9, 1)].fg, accent());
    assert!(!buffer[(9, 1)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(19, 1)].symbol(), "v");
    assert_eq!(buffer[(19, 1)].fg, chat_input_chrome());
    assert_eq!(buffer[(10, 2)].symbol(), " ");
    assert_eq!(buffer[(11, 3)].symbol(), "W");
}

#[test]
fn narrow_banner_uses_the_compact_single_column_copy() {
    let buffer = render(48, 12);
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("Welcome back!"));
    assert!(rendered.contains("/work/zeta"));
    assert!(rendered.contains("Use @ for files"));
    assert!(rendered.contains("Explain this directory"));
    assert!(!rendered.contains("╭─────╮"));
}

fn render(width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            draw(
                frame,
                frame.area(),
                &WelcomeModel::for_dir(Path::new("/work/zeta")),
                highlight(),
            )
        })
        .unwrap();
    terminal.backend().buffer().clone()
}
