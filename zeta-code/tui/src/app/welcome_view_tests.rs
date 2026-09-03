use super::WelcomeModel;
use super::draw;
use crate::models::ModelSummary;
use crate::render::test_context;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Modifier;
use std::path::Path;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_protocol::ModelAccess;

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
    assert!(rendered.contains(" ▜▙▄▄▄▟▛"));
    assert_eq!(
        buffer[(3, 1)].fg,
        ratatui::style::Color::Rgb(0x40, 0x85, 0xac)
    );
    assert_eq!(buffer[(4, 2)].bg, ratatui::style::Color::Rgb(0, 0, 0));
    assert_eq!(buffer[(14, 1)].symbol(), "Z");
    assert!(buffer[(14, 1)].modifier.contains(Modifier::BOLD));
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
fn generated_pet_cells_match_the_source_pixel_grid() {
    assert_snapshot!("welcome_pet_source_pixel_grid", pet_pixel_map());
}

fn pet_pixel_map() -> String {
    let sprite = super::pet::sprite();
    let width = usize::from(sprite.width()) * 2;
    let height = usize::from(sprite.height()) * 2;
    let mut rows = vec![vec!['.'; width]; height];
    for (index, cell) in sprite.cells().iter().copied().enumerate() {
        let cell_x = index % usize::from(sprite.width());
        let cell_y = index / usize::from(sprite.width());
        let mask = quadrant_mask(cell.symbol());
        for quadrant in 0..4 {
            let color = if mask & (1 << quadrant) != 0 {
                cell.foreground()
            } else {
                cell.background()
            };
            rows[cell_y * 2 + quadrant / 2][cell_x * 2 + quadrant % 2] = match color {
                None => '.',
                Some(ratatui::style::Color::Rgb(0x40, 0x85, 0xac)) => 'B',
                Some(ratatui::style::Color::Rgb(0, 0, 0)) => 'K',
                Some(color) => panic!("unexpected pet color {color:?}"),
            };
        }
    }
    rows.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn quadrant_mask(symbol: &str) -> u8 {
    match symbol {
        "" | " " => 0,
        "▘" => 1,
        "▝" => 2,
        "▀" => 3,
        "▖" => 4,
        "▌" => 5,
        "▞" => 6,
        "▛" => 7,
        "▗" => 8,
        "▚" => 9,
        "▐" => 10,
        "▜" => 11,
        "▄" => 12,
        "▙" => 13,
        "▟" => 14,
        "█" => 15,
        symbol => panic!("unexpected quadrant symbol {symbol}"),
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
