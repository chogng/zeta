use super::ComposerInteractionPaneState;
use zeta_ui::{Point, Rect, ScrollCommand, ScrollDelta, Size};

#[test]
fn pane_scrolls_from_only_viewport_and_content_geometry() {
    let mut pane = ComposerInteractionPaneState::default();

    assert!(pane.apply_scroll(
        ScrollCommand::ByPixels(ScrollDelta::vertical(70.0)),
        Size::new(300.0, 100.0),
        Size::new(300.0, 400.0),
    ));
    assert_eq!(pane.scroll_state().offset(), Point::new(0.0, 70.0));

    pane.reset();
    assert_eq!(pane.scroll_state(), Default::default());
}

#[test]
fn pane_reveals_arbitrary_mounted_content_bounds() {
    let mut pane = ComposerInteractionPaneState::default();

    assert!(pane.ensure_visible(
        Rect::from_xywh(0.0, 238.0, 300.0, 34.0),
        Size::new(300.0, 102.0),
        Size::new(300.0, 340.0),
    ));
    assert_eq!(pane.scroll_state().vertical_offset(), 170.0);
}
