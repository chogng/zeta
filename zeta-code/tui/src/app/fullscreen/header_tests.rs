use crate::app::App;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn header_keeps_workspace_visible_outside_the_transcript() {
    let app = App::for_dir(std::path::Path::new("/work/zeta"));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| crate::app::frame::draw(frame, &app))
        .unwrap();
    let row = terminal.backend().buffer().content[..80]
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(row.contains("Home  /work/zeta"));
    assert!(!row.contains("Zeta Code"));
}
