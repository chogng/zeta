use zeta_ui_components::InteractionRegion;
use zui::ui::AccessibilityRole;
use zui::ui::Border;
use zui::ui::Component;
use zui::ui::ComponentContext;
use zui::ui::ComponentElement;
use zui::ui::ComputedElement;
use zui::ui::CornerRadii;
use zui::ui::CursorFeedback;
use zui::ui::Element;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::FontFamily;
use zui::ui::FontWeight;
use zui::ui::NavigationAxis;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::PaintRect;
use zui::ui::Rect;
use zui::ui::TextBlock;
use zui::ui::TextStyle;
use zui::ui::UiDispatch;
use zui::ui::UiNode;
use zui::ui::UiScene;

use crate::{SETTINGS_PAGE, SettingsPageSection};

const SETTINGS_SECTION_SCOPE: u32 = 11;
const CONTENT_INSET_X: f32 = 38.0;
const CONTENT_INSET_TOP: f32 = 32.0;
const CONTENT_INSET_BOTTOM: f32 = 28.0;
const CARD_GAP: f32 = 12.0;
const ROW_HEIGHT: f32 = 36.0;

pub const SETTINGS_SECTION_CONTENT: ElementId = ElementId::scoped(SETTINGS_SECTION_SCOPE, 1);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsSectionStyle {
    pub background: zui::ui::Color,
    pub surface: zui::ui::Color,
    pub surface_raised: zui::ui::Color,
    pub surface_hovered: zui::ui::Color,
    pub border: zui::ui::Color,
    pub text: zui::ui::Color,
    pub text_muted: zui::ui::Color,
    pub accent: zui::ui::Color,
    pub error: zui::ui::Color,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsKeybindingRow {
    pub element: ElementId,
    pub label: String,
    pub value: String,
}

pub struct SettingsSectionPane<'a> {
    bounds: Rect,
    section: SettingsPageSection,
    style: SettingsSectionStyle,
    workspace_label: &'a str,
    connection_label: &'a str,
    surface_label: &'a str,
    keybinding_rows: &'a [SettingsKeybindingRow],
    keyboard_shortcuts_visible: bool,
    keybinding_diagnostics: &'a [String],
    theme_scheme: &'a str,
    theme_follows_system: bool,
    dispatch: &'a UiDispatch,
}

impl<'a> SettingsSectionPane<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bounds: Rect,
        section: SettingsPageSection,
        style: SettingsSectionStyle,
        workspace_label: &'a str,
        connection_label: &'a str,
        surface_label: &'a str,
        keybinding_rows: &'a [SettingsKeybindingRow],
        keyboard_shortcuts_visible: bool,
        keybinding_diagnostics: &'a [String],
        theme_scheme: &'a str,
        theme_follows_system: bool,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            bounds,
            section,
            style,
            workspace_label,
            connection_label,
            surface_label,
            keybinding_rows,
            keyboard_shortcuts_visible,
            keybinding_diagnostics,
            theme_scheme,
            theme_follows_system,
            dispatch,
        }
    }

    fn keybinding_row_bounds(&self, index: usize) -> Rect {
        let content = self.content_bounds();
        Rect::from_xywh(
            content.origin.x,
            content.origin.y + 92.0 + index as f32 * ROW_HEIGHT,
            content.size.width,
            ROW_HEIGHT,
        )
    }

    fn content_bounds(&self) -> Rect {
        Rect::from_xywh(
            self.bounds.origin.x + CONTENT_INSET_X,
            self.bounds.origin.y + CONTENT_INSET_TOP,
            (self.bounds.size.width - CONTENT_INSET_X * 2.0).max(1.0),
            (self.bounds.size.height - CONTENT_INSET_TOP - CONTENT_INSET_BOTTOM).max(1.0),
        )
    }

    fn interaction_regions(&self) -> Vec<InteractionRegion> {
        if self.section != SettingsPageSection::Keybindings || self.keyboard_shortcuts_visible {
            return Vec::new();
        }
        let navigation = NavigationGroupId::new(SETTINGS_SECTION_CONTENT);
        self.keybinding_rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                InteractionRegion::new(
                    "SettingsKeybindingRow",
                    row.element,
                    self.keybinding_row_bounds(index),
                    AccessibilityRole::Button,
                    format!("Record shortcut for {}", row.label),
                )
                .with_parent(SETTINGS_SECTION_CONTENT)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Vertical)
                .with_value(row.value.clone())
            })
            .collect()
    }

    fn paint_section(&self, scene: &mut UiScene) {
        scene.with_clip(self.bounds, |scene| match self.section {
            SettingsPageSection::General => self.paint_general(scene),
            SettingsPageSection::Appearance => self.paint_appearance(scene),
            SettingsPageSection::Keybindings => self.paint_keybindings(scene),
        });
    }

    fn paint_general(&self, scene: &mut UiScene) {
        self.paint_header(
            scene,
            "General",
            "Workspace and session defaults for this app window.",
        );
        let rows = [
            ("Workspace", self.workspace_label.to_owned()),
            ("Connection", self.connection_label.to_owned()),
            ("Surface", self.surface_label.to_owned()),
        ];
        self.paint_value_card(scene, 92.0, &rows);
        self.paint_note(
            scene,
            92.0 + card_height(rows.len()),
            "General preferences are projected from the active workspace. Persistent controls will be added here as their configuration authority is defined.",
        );
    }

    fn paint_appearance(&self, scene: &mut UiScene) {
        self.paint_header(
            scene,
            "Appearance",
            "Theme and visual language used by the current window.",
        );
        let theme = if self.theme_follows_system {
            format!("System ({})", self.theme_scheme)
        } else {
            format!("Custom ({})", self.theme_scheme)
        };
        let rows = [
            ("Theme", theme),
            ("Accent", "Current workspace accent".to_owned()),
        ];
        self.paint_value_card(scene, 92.0, &rows);
        let swatch_y = self.content_bounds().origin.y + 92.0 + card_height(rows.len()) + 24.0;
        let swatch_bounds = Rect::from_xywh(
            self.content_bounds().origin.x,
            swatch_y,
            self.content_bounds().size.width,
            74.0,
        );
        scene.draw_rect(
            PaintRect::new(swatch_bounds, self.style.surface)
                .with_border(Border::uniform(1.0, self.style.border))
                .with_corner_radii(CornerRadii::uniform(6.0)),
        );
        draw_label(
            scene,
            "Current palette",
            Rect::from_xywh(
                swatch_bounds.origin.x + 14.0,
                swatch_bounds.origin.y + 12.0,
                160.0,
                18.0,
            ),
            TextStyle::new(12.0, self.style.text_muted).with_line_height(18.0),
        );
        for (index, color) in [
            self.style.background,
            self.style.surface,
            self.style.surface_raised,
            self.style.accent,
        ]
        .into_iter()
        .enumerate()
        {
            scene.draw_rect(
                PaintRect::new(
                    Rect::from_xywh(
                        swatch_bounds.origin.x + 14.0 + index as f32 * 42.0,
                        swatch_bounds.origin.y + 38.0,
                        30.0,
                        22.0,
                    ),
                    color,
                )
                .with_border(Border::uniform(1.0, self.style.border))
                .with_corner_radii(CornerRadii::uniform(4.0)),
            );
        }
    }

    fn paint_keybindings(&self, scene: &mut UiScene) {
        self.paint_header(
            scene,
            "Keybindings",
            "Current commands and shortcuts. Select a row to record a new shortcut.",
        );
        for (index, row) in self.keybinding_rows.iter().enumerate() {
            let bounds = self.keybinding_row_bounds(index);
            if self.dispatch.is_hovered(row.element)
                || self.dispatch.is_focused(row.element)
                || self.dispatch.is_pressed(row.element)
            {
                scene.draw_rect(
                    PaintRect::new(bounds, self.style.surface_hovered)
                        .with_corner_radii(CornerRadii::uniform(4.0)),
                );
            }
            draw_label(
                scene,
                &row.label,
                Rect::from_xywh(
                    bounds.origin.x + 10.0,
                    bounds.origin.y + 8.0,
                    bounds.size.width * 0.55,
                    20.0,
                ),
                TextStyle::new(13.0, self.style.text).with_line_height(20.0),
            );
            draw_label(
                scene,
                &row.value,
                Rect::from_xywh(
                    bounds.origin.x + bounds.size.width * 0.58,
                    bounds.origin.y + 8.0,
                    bounds.size.width * 0.4,
                    20.0,
                ),
                TextStyle::new(12.0, self.style.text_muted)
                    .with_family(FontFamily::Monospace)
                    .with_line_height(20.0),
            );
        }
        if let Some(diagnostic) = self.keybinding_diagnostics.first() {
            draw_label(
                scene,
                diagnostic,
                Rect::from_xywh(
                    self.content_bounds().origin.x,
                    self.content_bounds().bottom() - 24.0,
                    self.content_bounds().size.width,
                    20.0,
                ),
                TextStyle::new(12.0, self.style.error).with_line_height(20.0),
            );
        }
    }

    fn paint_header(&self, scene: &mut UiScene, title: &str, description: &str) {
        let content = self.content_bounds();
        draw_label(
            scene,
            title,
            Rect::from_xywh(content.origin.x, content.origin.y, content.size.width, 30.0),
            TextStyle::new(22.0, self.style.text)
                .with_weight(FontWeight::Bold)
                .with_line_height(30.0),
        );
        draw_label(
            scene,
            description,
            Rect::from_xywh(
                content.origin.x,
                content.origin.y + 38.0,
                content.size.width,
                22.0,
            ),
            TextStyle::new(13.0, self.style.text_muted).with_line_height(22.0),
        );
    }

    fn paint_value_card(&self, scene: &mut UiScene, y_offset: f32, rows: &[(&str, String)]) {
        let content = self.content_bounds();
        let bounds = Rect::from_xywh(
            content.origin.x,
            content.origin.y + y_offset,
            content.size.width,
            card_height(rows.len()),
        );
        scene.draw_rect(
            PaintRect::new(bounds, self.style.surface)
                .with_border(Border::uniform(1.0, self.style.border))
                .with_corner_radii(CornerRadii::uniform(6.0)),
        );
        for (index, (label, value)) in rows.iter().enumerate() {
            let row_y = bounds.origin.y + 12.0 + index as f32 * ROW_HEIGHT;
            draw_label(
                scene,
                label,
                Rect::from_xywh(bounds.origin.x + 14.0, row_y, 150.0, 20.0),
                TextStyle::new(12.0, self.style.text_muted).with_line_height(20.0),
            );
            draw_label(
                scene,
                value,
                Rect::from_xywh(
                    bounds.origin.x + 168.0,
                    row_y,
                    (bounds.size.width - 182.0).max(1.0),
                    20.0,
                ),
                TextStyle::new(13.0, self.style.text).with_line_height(20.0),
            );
        }
    }

    fn paint_note(&self, scene: &mut UiScene, y_offset: f32, note: &str) {
        let content = self.content_bounds();
        draw_label(
            scene,
            note,
            Rect::from_xywh(
                content.origin.x,
                content.origin.y + y_offset + CARD_GAP,
                content.size.width,
                48.0,
            ),
            TextStyle::new(12.0, self.style.text_muted).with_line_height(18.0),
        );
    }
}

impl Component for SettingsSectionPane<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("SettingsSectionPane")
            .in_bounds(self.bounds)
            .with_identity(SETTINGS_SECTION_CONTENT)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                SETTINGS_SECTION_CONTENT,
                element.bounds(),
                AccessibilityRole::Group,
                self.section_label(),
            )
            .with_parent(SETTINGS_PAGE),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.interaction_regions() {
            context.draw_component(&region);
        }
        self.paint_section(context.scene_mut());
    }

    fn paint(&self, scene: &mut UiScene) {
        self.paint_section(scene);
    }
}

impl SettingsSectionPane<'_> {
    fn section_label(&self) -> &'static str {
        match self.section {
            SettingsPageSection::General => "General settings",
            SettingsPageSection::Appearance => "Appearance settings",
            SettingsPageSection::Keybindings => "Keybinding settings",
        }
    }
}

fn card_height(row_count: usize) -> f32 {
    24.0 + row_count as f32 * ROW_HEIGHT
}

fn draw_label(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(
        text.to_owned(),
        bounds.origin,
        bounds.size,
        style,
    ));
}
