use super::ChatComposerSurface;
use super::areas;
use super::desired_height;
use super::view_areas;
use crate::app::App;
use crate::components::chat_composer::ChatComposerPaneKind;
use crate::components::chat_input::ChatInputCursor;
use crate::render::Renderable;
use crate::render::test_context;
use ratatui::layout::Rect;

#[test]
fn pane_consumes_the_persistent_input_area() {
    let entries = [(ChatComposerPaneKind::Stacked, 9)];
    let areas = areas(Rect::new(0, 10, 80, 12), 3, &entries);

    assert_eq!(areas.panes[0].area, Rect::new(0, 10, 80, 12));
    assert_eq!(areas.input, Rect::new(0, 22, 80, 0));
    assert_eq!(desired_height(3, &entries), 12);
}

#[test]
fn clamped_pane_keeps_the_input_hidden() {
    let entries = [(ChatComposerPaneKind::Stacked, 99)];
    let areas = areas(Rect::new(0, 10, 80, 7), 3, &entries);

    assert_eq!(areas.panes[0].area, Rect::new(0, 10, 80, 7));
    assert_eq!(areas.input, Rect::new(0, 17, 80, 0));
}

#[test]
fn renderable_measurement_matches_the_composer_area_allocation() {
    let app = App::new();
    let view = app.chat_composer_view();
    let surface = ChatComposerSurface {
        overlay_area: Rect::default(),
        view: &view,
        cursor: ChatInputCursor::Hidden,
        hovered: None,
        pressed: None,
    };

    let height = surface.desired_height(80, test_context());
    let allocated = view_areas(Rect::new(0, 0, 80, height), &view);

    assert_eq!(allocated.input.bottom(), height);
}
