use super::SessionCanvasLayout;
use super::SessionHeader;
use crate::shell_style::SHELL_PALETTE;
use crate::thread_projection::ThreadProjection;
use crate::workspace_context::WorkspaceContext;
use zeta_ui::Component;
use zeta_ui::Rect;
use zeta_ui::UiScene;

#[test]
fn session_canvas_reserves_a_stable_header_above_the_thread_timeline() {
    let output = Rect::from_xywh(200.0, 32.0, 600.0, 500.0);

    let layout = SessionCanvasLayout::for_output(output);

    assert_eq!(layout.header(), Rect::from_xywh(200.0, 32.0, 600.0, 64.0));
    assert_eq!(
        layout.timeline(),
        Rect::from_xywh(200.0, 96.0, 600.0, 436.0)
    );
}

#[test]
fn empty_session_header_shows_ready_workspace_context() {
    let projection = ThreadProjection::default();
    let workspace = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(2));
    let header = SessionHeader::new(
        Rect::from_xywh(0.0, 0.0, 700.0, 64.0),
        "",
        &projection,
        &workspace,
        SHELL_PALETTE,
    );
    let mut scene = UiScene::new(SHELL_PALETTE.background);

    header.paint(&mut scene);

    let text = scene
        .text_blocks()
        .iter()
        .map(|block| block.text())
        .collect::<Vec<_>>();
    assert!(text.contains(&"New session"));
    assert!(text.contains(&"Ready"));
    assert!(
        text.iter()
            .any(|line| line.contains("~/Desktop/zeta") && line.contains("Changes 2"))
    );
}
