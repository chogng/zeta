use super::WelcomeModel;
use super::draw;
use crate::models::ModelSummary;
use crate::render::test_context;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;
use std::path::Path;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_protocol::ModelAccess;
use zeta_sprite::Rgb;
use zeta_sprite::SpriteCell;
use zeta_sprite::TerminalSprite;

#[test]
fn wide_header_keeps_pet_and_identity_information_together() {
    let mut model = WelcomeModel::for_workspace(Path::new("/work/zeta"));
    model.apply_model_summary(&ModelSummary::from_catalog(
        Some(ModelRefDto {
            provider: "openai-chatgpt".into(),
            model: "gpt-5.6".into(),
        }),
        None,
    ));
    model.access = ModelAccess::Subscription;
    let buffer = render(80, 10, &model);
    let rendered = buffer_text(&buffer, 80, 10);

    assert!(rendered.contains(concat!("Zeta Code v", env!("CARGO_PKG_VERSION"))));
    assert!(rendered.contains("openai-chatgpt/gpt-5.6 · Subscription"));
    assert!(rendered.contains("/work/zeta"));
    assert_eq!(
        (super::pet::sprite().width(), super::pet::sprite().height()),
        (8, 4)
    );
    assert_eq!(super::desired_height(80), 5);
    assert_eq!(buffer[(4, 2)].symbol(), "▛");
    assert_eq!(buffer[(4, 2)].fg, Color::Rgb(0x40, 0x85, 0xac));
    assert_eq!(buffer[(4, 2)].bg, Color::Rgb(0, 0, 0));
    assert_eq!(buffer[(13, 1)].symbol(), "Z");
    assert!(buffer[(13, 1)].modifier.contains(Modifier::BOLD));
    assert_snapshot!("welcome_pet_identity_header", rendered);
}

#[test]
fn narrow_header_keeps_the_text_alternative_when_the_pet_does_not_fit() {
    let model = WelcomeModel::for_workspace(Path::new("/zeta"));
    let buffer = render(20, 5, &model);
    let rendered = buffer_text(&buffer, 20, 5);

    assert!(rendered.contains("Zeta Code"));
    assert!(rendered.contains("Automatic model"));
    assert!(rendered.contains("/zeta"));
    assert!(!rendered.contains("██"));
}

#[test]
fn generated_pet_cells_preserve_the_authored_terminal_instructions() {
    assert_snapshot!(
        "welcome_pet_terminal_cells",
        pet_cell_map(super::pet::sprite())
    );
}

#[test]
fn background_colored_spaces_are_rendered() {
    let cells = [SpriteCell::new(' ', None, Some(Rgb::new(0x40, 0x85, 0xac)))];
    let sprite = TerminalSprite::new(1, 1, &cells);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 1));

    super::pet::PetWidget::new(sprite).render(buffer.area, &mut buffer);

    assert_eq!(buffer[(0, 0)].symbol(), " ");
    assert_eq!(buffer[(0, 0)].bg, Color::Rgb(0x40, 0x85, 0xac));
}

#[test]
fn generated_pet_frames_and_click_timing_match_the_design() {
    let sheet = super::pet::sheet();
    let frames = sheet
        .frames()
        .iter()
        .map(|frame| format!("{}\n{}", frame.name(), pet_cell_map(frame.sprite())))
        .collect::<Vec<_>>()
        .join("\n\n");
    let click = sheet
        .actions()
        .iter()
        .find(|action| action.name() == "click")
        .unwrap();
    let timing = click
        .steps()
        .iter()
        .map(|step| {
            format!(
                "{} {}ms",
                sheet.frames()[usize::from(step.frame_index())].name(),
                step.duration_ms()
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(timing, ["press 75ms", "rise 100ms", "land 100ms"]);
    assert_snapshot!("welcome_pet_animation_frames", frames);
}

fn pet_cell_map(sprite: TerminalSprite<'_>) -> String {
    let symbols = cell_rows(sprite, |cell| match cell.symbol() {
        ' ' => '.',
        symbol => symbol,
    });
    let foreground = cell_rows(sprite, |cell| color_symbol(cell.foreground()));
    let background = cell_rows(sprite, |cell| color_symbol(cell.background()));
    format!("symbols\n{symbols}\nforeground\n{foreground}\nbackground\n{background}")
}

fn cell_rows(sprite: TerminalSprite<'_>, value: impl Fn(SpriteCell) -> char) -> String {
    sprite
        .cells()
        .chunks_exact(usize::from(sprite.width()))
        .map(|row| row.iter().copied().map(&value).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn color_symbol(color: Option<Rgb>) -> char {
    match color {
        None => '.',
        Some(color) if color.components() == [0x40, 0x85, 0xac] => 'B',
        Some(color) if color.components() == [0, 0, 0] => 'K',
        Some(color) => panic!("unexpected pet color {color:?}"),
    }
}

fn render(width: u16, height: u16, model: &WelcomeModel) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| draw(frame, frame.area(), model, test_context()))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn buffer_text(buffer: &Buffer, width: u16, height: u16) -> String {
    (0..height)
        .map(|row| {
            (0..width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
