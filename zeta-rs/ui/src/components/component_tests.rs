use super::Component;
use crate::{Color, ComponentInspection, PaintRect, Rect, UiScene};

struct TestComponent {
    bounds: Rect,
}

impl Component for TestComponent {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("TestComponent", self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, Color::WHITE));
    }
}

#[test]
fn scene_draws_component_inside_active_clip() {
    let component = TestComponent {
        bounds: Rect::from_xywh(0.0, 0.0, 40.0, 40.0),
    };
    let mut scene = UiScene::new(Color::TRANSPARENT);

    scene.with_clip(Rect::from_xywh(10.0, 12.0, 20.0, 16.0), |scene| {
        scene.draw_component(&component);
    });

    assert_eq!(scene.rects().len(), 1);
    assert_eq!(scene.inspection().nodes()[0].name(), "TestComponent");
    assert_eq!(
        scene.rects()[0].clip_bounds(),
        Some(Rect::from_xywh(10.0, 12.0, 20.0, 16.0))
    );
}
