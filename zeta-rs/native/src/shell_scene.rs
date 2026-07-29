use zeta_ui::{
    Border, Color, CornerRadii, Edges, FontWeight, PaintIcon, PaintRect, Rect, SvgIcon, TextBlock,
    TextStyle, UiScene,
};

use crate::shell_interaction::{SessionId, ShellHitMap, ShellInteraction, ShellTarget};
use crate::shell_theme::ShellPalette;

const TITLEBAR_HEIGHT: f32 = 35.0;
const SIDEBAR_TARGET_WIDTH: f32 = 232.0;
const COMPOSER_HEIGHT: f32 = 68.0;
const THEME_ICON: SvgIcon = SvgIcon::new("theme", include_bytes!("../assets/theme.svg"));

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LogicalViewport {
    pub width: f32,
    pub height: f32,
}

impl LogicalViewport {
    pub(crate) fn from_physical(width: u32, height: u32, scale_factor: f64) -> Self {
        let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor as f32
        } else {
            1.0
        };
        Self {
            width: width as f32 / scale_factor,
            height: height as f32 / scale_factor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShellLayout {
    titlebar: Rect,
    sidebar: Rect,
    main: Rect,
    transcript: Rect,
    composer: Rect,
}

impl ShellLayout {
    fn for_viewport(viewport: LogicalViewport) -> Option<Self> {
        if viewport.width < 520.0 || viewport.height < 300.0 {
            return None;
        }
        let titlebar = Rect::from_xywh(0.0, 0.0, viewport.width, TITLEBAR_HEIGHT);
        let body_height = viewport.height - titlebar.size.height;
        let sidebar_width = SIDEBAR_TARGET_WIDTH.min((viewport.width * 0.32).max(176.0));
        let sidebar = Rect::from_xywh(0.0, titlebar.bottom(), sidebar_width, body_height);
        let main = Rect::from_xywh(
            sidebar.right(),
            titlebar.bottom(),
            viewport.width - sidebar_width,
            body_height,
        );
        let composer_margin = 20.0_f32.min(main.size.width * 0.08);
        let composer = Rect::from_xywh(
            main.origin.x + composer_margin,
            main.bottom() - COMPOSER_HEIGHT - composer_margin,
            (main.size.width - composer_margin * 2.0).max(0.0),
            COMPOSER_HEIGHT,
        );
        let transcript = Rect::from_xywh(
            main.origin.x,
            main.origin.y,
            main.size.width,
            (composer.origin.y - main.origin.y - 12.0).max(0.0),
        );
        Some(Self {
            titlebar,
            sidebar,
            main,
            transcript,
            composer,
        })
    }
}

pub(crate) struct ShellPresentation {
    pub(crate) scene: UiScene,
    pub(crate) hit_map: ShellHitMap,
}

pub(crate) fn build_shell_presentation(
    viewport: LogicalViewport,
    interaction: &ShellInteraction,
) -> ShellPresentation {
    let palette = interaction.theme().palette();
    let mut scene = UiScene::new(palette.background);
    let mut hit_map = ShellHitMap::default();
    let Some(layout) = ShellLayout::for_viewport(viewport) else {
        draw_compact_scene(&mut scene, viewport, palette);
        return ShellPresentation { scene, hit_map };
    };

    draw_titlebar(&mut scene, &mut hit_map, layout, interaction, palette);
    draw_sidebar(&mut scene, &mut hit_map, layout, interaction, palette);
    draw_main(&mut scene, &mut hit_map, layout, interaction, palette);
    ShellPresentation { scene, hit_map }
}

fn draw_titlebar(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    interaction: &ShellInteraction,
    palette: ShellPalette,
) {
    scene.draw_rect(
        PaintRect::new(layout.titlebar, palette.surface_raised)
            .with_border(Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), palette.border)),
    );
    hit_map.register(layout.titlebar, ShellTarget::WindowDrag);
    draw_text(
        scene,
        "Zeta",
        Rect::from_xywh(78.0, 7.0, 180.0, 22.0),
        TextStyle::new(17.0, palette.text).with_weight(FontWeight::Bold),
    );

    let theme_button = Rect::from_xywh(layout.titlebar.right() - 112.0, 4.0, 100.0, 27.0);
    let target = ShellTarget::ThemeToggle;
    hit_map.register(theme_button, target);
    scene.draw_rect(
        PaintRect::new(
            theme_button,
            interactive_fill(interaction, target, palette.surface, palette),
        )
        .with_border(Border::uniform(1.0, palette.border))
        .with_corner_radii(CornerRadii::uniform(8.0)),
    );
    scene.draw_icon(PaintIcon::new(
        THEME_ICON,
        Rect::from_xywh(
            theme_button.origin.x + 10.0,
            theme_button.origin.y + 7.0,
            13.0,
            13.0,
        ),
        palette.accent,
    ));
    draw_text(
        scene,
        interaction.theme().toggle_label(),
        Rect::from_xywh(
            theme_button.origin.x + 29.0,
            theme_button.origin.y + 6.0,
            theme_button.size.width - 37.0,
            17.0,
        ),
        TextStyle::new(11.0, palette.accent).with_weight(FontWeight::Bold),
    );
}

fn draw_sidebar(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    interaction: &ShellInteraction,
    palette: ShellPalette,
) {
    scene.draw_rect(
        PaintRect::new(layout.sidebar, palette.surface)
            .with_border(Border::new(Edges::new(0.0, 1.0, 0.0, 0.0), palette.border)),
    );
    scene.with_clip(layout.sidebar, |scene| {
        draw_text(
            scene,
            "SESSIONS",
            Rect::from_xywh(
                layout.sidebar.origin.x + 18.0,
                layout.sidebar.origin.y + 20.0,
                layout.sidebar.size.width - 36.0,
                20.0,
            ),
            TextStyle::new(11.0, palette.text_muted).with_weight(FontWeight::Bold),
        );
        for (index, session) in [
            SessionId::Foundation,
            SessionId::Renderer,
            SessionId::AppServer,
        ]
        .into_iter()
        .enumerate()
        {
            let row = Rect::from_xywh(
                layout.sidebar.origin.x + 14.0,
                layout.sidebar.origin.y + 52.0 + index as f32 * 64.0,
                layout.sidebar.size.width - 28.0,
                52.0,
            );
            draw_session_row(scene, hit_map, row, session, interaction, palette);
        }
    });
}

fn draw_session_row(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    row: Rect,
    session: SessionId,
    interaction: &ShellInteraction,
    palette: ShellPalette,
) {
    let target = ShellTarget::Session(session);
    hit_map.register(row, target);
    let resting = if interaction.selected_session() == session {
        palette.surface_selected
    } else {
        palette.surface
    };
    scene.draw_rect(
        PaintRect::new(row, interactive_fill(interaction, target, resting, palette))
            .with_border(Border::uniform(
                1.0,
                if interaction.selected_session() == session {
                    palette.border_focused
                } else {
                    Color::TRANSPARENT
                },
            ))
            .with_corner_radii(CornerRadii::uniform(8.0)),
    );
    let (title, subtitle) = session_labels(session);
    draw_text(
        scene,
        title,
        Rect::from_xywh(
            row.origin.x + 12.0,
            row.origin.y + 8.0,
            row.size.width - 24.0,
            20.0,
        ),
        TextStyle::new(13.0, palette.text).with_weight(FontWeight::Bold),
    );
    draw_text(
        scene,
        subtitle,
        Rect::from_xywh(
            row.origin.x + 12.0,
            row.origin.y + 29.0,
            row.size.width - 24.0,
            17.0,
        ),
        TextStyle::new(11.0, palette.text_muted),
    );
}

fn draw_main(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    interaction: &ShellInteraction,
    palette: ShellPalette,
) {
    scene.draw_rect(PaintRect::new(layout.main, palette.background));
    scene.with_clip(layout.main, |scene| {
        let (heading, summary, message) = session_content(interaction.selected_session());
        draw_text(
            scene,
            heading,
            Rect::from_xywh(
                layout.main.origin.x + 24.0,
                layout.main.origin.y + 22.0,
                layout.main.size.width - 48.0,
                30.0,
            ),
            TextStyle::new(18.0, palette.text).with_weight(FontWeight::Bold),
        );
        draw_text(
            scene,
            summary,
            Rect::from_xywh(
                layout.main.origin.x + 24.0,
                layout.main.origin.y + 54.0,
                layout.main.size.width - 48.0,
                44.0,
            ),
            TextStyle::new(14.0, palette.text_muted),
        );
        draw_message_card(scene, layout, message, palette);
        draw_composer(scene, hit_map, layout, interaction, palette);
    });
}

fn draw_message_card(
    scene: &mut UiScene,
    layout: ShellLayout,
    message_text: &str,
    palette: ShellPalette,
) {
    let message = Rect::from_xywh(
        layout.transcript.origin.x + 24.0,
        layout.transcript.origin.y + 108.0,
        (layout.transcript.size.width - 48.0).clamp(0.0, 560.0),
        92.0,
    );
    scene.draw_rect(
        PaintRect::new(message, palette.surface_raised)
            .with_border(Border::uniform(1.0, palette.border))
            .with_corner_radii(CornerRadii::uniform(10.0)),
    );
    draw_text(
        scene,
        "Zeta",
        Rect::from_xywh(
            message.origin.x + 16.0,
            message.origin.y + 13.0,
            message.size.width - 32.0,
            22.0,
        ),
        TextStyle::new(13.0, palette.accent).with_weight(FontWeight::Bold),
    );
    draw_text(
        scene,
        message_text,
        Rect::from_xywh(
            message.origin.x + 16.0,
            message.origin.y + 40.0,
            message.size.width - 32.0,
            42.0,
        ),
        TextStyle::new(14.0, palette.text),
    );
}

fn draw_composer(
    scene: &mut UiScene,
    hit_map: &mut ShellHitMap,
    layout: ShellLayout,
    interaction: &ShellInteraction,
    palette: ShellPalette,
) {
    scene.draw_rect(PaintRect::new(
        Rect::from_xywh(
            layout.main.origin.x,
            layout.composer.origin.y - 12.0,
            layout.main.size.width,
            1.0,
        ),
        palette.border,
    ));
    let target = ShellTarget::Composer;
    hit_map.register(layout.composer, target);
    scene.draw_rect(
        PaintRect::new(
            layout.composer,
            interactive_fill(interaction, target, palette.surface_raised, palette),
        )
        .with_border(Border::uniform(
            1.0,
            if interaction.composer_focused() {
                palette.accent
            } else {
                palette.border
            },
        ))
        .with_corner_radii(CornerRadii::uniform(11.0)),
    );
    scene.with_clip(layout.composer, |scene| {
        draw_text(
            scene,
            if interaction.composer_focused() {
                "Composer focused"
            } else {
                "Click to focus…"
            },
            Rect::from_xywh(
                layout.composer.origin.x + 18.0,
                layout.composer.origin.y + 22.0,
                layout.composer.size.width - 36.0,
                28.0,
            ),
            TextStyle::new(15.0, palette.text_muted),
        );
    });
}

fn interactive_fill(
    interaction: &ShellInteraction,
    target: ShellTarget,
    resting: Color,
    palette: ShellPalette,
) -> Color {
    if interaction.is_pressed(target) {
        palette.surface_pressed
    } else if interaction.is_hovered(target) {
        palette.surface_hovered
    } else {
        resting
    }
}

fn session_labels(session: SessionId) -> (&'static str, &'static str) {
    match session {
        SessionId::Foundation => ("Native UI foundation", "Active now"),
        SessionId::Renderer => ("Renderer architecture", "GPU scene"),
        SessionId::AppServer => ("App Server integration", "Planned"),
    }
}

fn session_content(session: SessionId) -> (&'static str, &'static str, &'static str) {
    match session {
        SessionId::Foundation => (
            "Conversation",
            "Rect, border, rounded corners and clipping share one scene.",
            "The native shell now owns hover, pressed, selected and focus state.",
        ),
        SessionId::Renderer => (
            "Renderer architecture",
            "Logical scene primitives are prepared into physical GPU instances.",
            "The renderer stays presentation-only; product interaction remains in the host.",
        ),
        SessionId::AppServer => (
            "App Server integration",
            "Product state will arrive through the typed App Server client contract.",
            "This preview does not duplicate Session, Thread or Turn state machines.",
        ),
    }
}

fn draw_compact_scene(scene: &mut UiScene, viewport: LogicalViewport, palette: ShellPalette) {
    let bounds = Rect::from_xywh(
        12.0,
        12.0,
        (viewport.width - 24.0).max(1.0),
        (viewport.height - 24.0).max(1.0),
    );
    scene.draw_rect(
        PaintRect::new(bounds, palette.surface)
            .with_border(Border::uniform(1.0, palette.border))
            .with_corner_radii(CornerRadii::uniform(10.0)),
    );
    draw_text(
        scene,
        "Zeta Native",
        Rect::from_xywh(
            bounds.origin.x + 18.0,
            bounds.origin.y + 18.0,
            (bounds.size.width - 36.0).max(1.0),
            30.0,
        ),
        TextStyle::new(20.0, palette.text).with_weight(FontWeight::Bold),
    );
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    if bounds.is_empty() {
        return;
    }
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}

#[cfg(test)]
#[path = "shell_scene_tests.rs"]
mod tests;
