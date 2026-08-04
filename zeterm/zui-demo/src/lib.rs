//! Minimal host proving that the reusable UI stack can run without zeterm product state.

use zeta_renderer::RenderOutcome;
use zeta_renderer::RenderTargetSize;
use zeta_renderer::Renderer;
use zeta_renderer::RendererError;
use zeta_ui::ActionBar;
use zeta_ui::ActionBarButton;
use zeta_ui::ActionBarItem;
use zeta_ui::ActionBarOrientation;
use zeta_ui::ActionBarStyle;
use zeta_ui::ButtonBackgrounds;
use zeta_ui::ButtonState;
use zeta_ui::ButtonStyle;
use zeta_ui::Color;
use zeta_ui::Edges;
use zeta_ui::Size;
use zeta_ui::TextStyle;
use zui::Icon;
use zui::IconDefinition;
use zui::IconId;
use zui::InteractionFrame;
use zui::UiFrame;
use zui::UiScene;

const DEMO_ICON: Icon = Icon::new(
    IconId::new("demo-square"),
    IconDefinition::symbolic(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="3" y="3" width="10" height="10"/></svg>"#,
    ),
);

/// Counts the backend-neutral scene frames consumed by the demo renderer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DemoRenderStats {
    /// Number of scenes submitted to the renderer.
    pub scene_count: usize,
    /// Number of retained rectangle primitives in the last submitted scene.
    pub rect_count: usize,
    /// Number of retained icon primitives in the last submitted scene.
    pub icon_count: usize,
    /// Number of retained text blocks in the last submitted scene.
    pub text_count: usize,
}

#[derive(Default)]
struct RecordingRenderer {
    stats: DemoRenderStats,
}

impl Renderer for RecordingRenderer {
    fn resize(&mut self, _size: RenderTargetSize) {}

    fn set_scale_factor(&mut self, _scale_factor: f64) {}

    fn render(&mut self) -> Result<RenderOutcome, RendererError> {
        Ok(RenderOutcome::Presented)
    }

    fn render_scene(&mut self, scene: &UiScene) -> Result<RenderOutcome, RendererError> {
        self.stats.scene_count += 1;
        self.stats.rect_count = scene.rects().len();
        self.stats.icon_count = scene.icons().len();
        self.stats.text_count = scene.text_blocks().len();
        Ok(RenderOutcome::Presented)
    }
}

/// Builds a component-only frame using caller-provided icon artwork and no product state.
pub fn build_demo_frame() -> UiFrame<InteractionFrame> {
    let button_style = ButtonStyle::new(
        ButtonBackgrounds::new(Color::rgb(35, 42, 52)).with_hovered(Color::rgb(48, 58, 70)),
        TextStyle::new(13.0, Color::WHITE),
    )
    .with_padding(Edges::uniform(8.0))
    .with_icon_size(14.0)
    .with_content_gap(6.0);
    let action_bar = ActionBar::new(
        zui::Rect::from_xywh(16.0, 16.0, 220.0, 32.0),
        ActionBarOrientation::Horizontal,
        vec![ActionBarItem::Button(ActionBarButton::icon_and_label(
            DEMO_ICON,
            "Open",
            ButtonState::Resting,
        ))],
        ActionBarStyle::new(button_style, Size::new(104.0, 32.0)),
    );
    let mut frame = UiFrame::<InteractionFrame>::new(Color::rgb(18, 22, 28));
    frame.draw_component(&action_bar);
    frame
}

/// Runs the framework-only demo through a replaceable backend-neutral renderer.
pub fn render_demo() -> Result<DemoRenderStats, RendererError> {
    let frame = build_demo_frame();
    let mut renderer = RecordingRenderer::default();
    renderer.render_scene(frame.scene())?;
    Ok(renderer.stats)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
