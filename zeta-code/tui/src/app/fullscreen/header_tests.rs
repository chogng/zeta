use crate::app::App;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use zeta_app_server_protocol::protocol::git::GitHeadDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;

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
    assert!(row.starts_with("  ≡ /work/zeta"));
    assert!(!row.contains("Zeta Code"));
}

#[test]
fn header_places_branch_and_path_beside_the_menu_without_repeating_them_below() {
    let mut app = App::for_dir(std::path::Path::new("/work/zeta"));
    app.update(crate::status::Event::GitStatusReceived(GitStatusResult {
        repository_id: "repository".into(),
        stream_instance_id: zeta_protocol::StreamInstanceId::new("git-stream").unwrap(),
        revision: 1,
        path: "/work/zeta".into(),
        head: GitHeadDto::Unborn {
            name: "main".into(),
        },
        changes: vec![],
    }));
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal
        .draw(|frame| crate::app::frame::draw(frame, &app))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text = buffer
        .content
        .chunks(80)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.lines()
            .next()
            .unwrap()
            .starts_with("  ≡ main /work/zeta")
    );
    assert_eq!(text.matches("main").count(), 1);
    assert_eq!(text.matches("/work/zeta").count(), 1);
    assert_eq!(buffer[(4, 0)].fg, app.render_context().foreground());
    assert!(buffer[(4, 0)].modifier.contains(Modifier::BOLD));
    assert_eq!(buffer[(9, 0)].fg, app.render_context().muted());
    crate::tui_assert_snapshot!("workspace_header_and_hintbar", text);

    let area = Rect::new(0, 0, 80, 20);
    let header = super::super::layout(&app, area).header;
    assert!(super::home_at(header, Position::new(2, 0)));
    assert!(!super::home_at(header, Position::new(4, 0)));
    super::super::pointer::activate_pointer_item(&mut app, area, 2, 0);
    assert!(app.fullscreen_home_visible());
}
