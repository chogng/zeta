use zeta_keybinding::format_key_sequence;
use zeta_settings::SettingsPageSection;
use zeta_ui::Border;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::CornerRadii;
use zeta_ui::Element;
use zeta_ui::FontFamily;
use zeta_ui::FontWeight;
use zeta_ui::InteractionRegion;
use zeta_ui::PaintRect;
use zeta_ui::Rect;
use zeta_ui::TextBlock;
use zeta_ui::TextStyle;
use zeta_ui::UiScene;
use zui::ui::AccessibilityRole;
use zui::ui::CursorFeedback;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::NavigationAxis;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::UiDispatch;
use zui::ui::UiNode;

use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::{KeyboardShortcutsState, keyboard_shortcut_rows};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_surface::WorkspaceSurfaceKind;

const SETTINGS_SECTION_SCOPE: u32 = 11;
const CONTENT_INSET_X: f32 = 38.0;
const CONTENT_INSET_TOP: f32 = 32.0;
const CONTENT_INSET_BOTTOM: f32 = 28.0;
const CARD_GAP: f32 = 12.0;
const ROW_HEIGHT: f32 = 36.0;

pub(crate) const SETTINGS_SECTION_CONTENT: ElementId = ElementId::scoped(SETTINGS_SECTION_SCOPE, 1);

pub(crate) struct SettingsSectionPane<'a> {
    bounds: Rect,
    section: SettingsPageSection,
    palette: ShellPalette,
    workspace_context: &'a WorkspaceContext,
    workspace_surface: WorkspaceSurfaceKind,
    keybindings: &'a NativeKeybindings,
    keyboard_shortcuts: &'a KeyboardShortcutsState,
    keybinding_diagnostics: &'a [String],
    theme_scheme: zeta_theme::ColorScheme,
    theme_follows_system: bool,
    dispatch: &'a UiDispatch,
}

impl<'a> SettingsSectionPane<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        bounds: Rect,
        section: SettingsPageSection,
        palette: ShellPalette,
        workspace_context: &'a WorkspaceContext,
        workspace_surface: WorkspaceSurfaceKind,
        keybindings: &'a NativeKeybindings,
        keyboard_shortcuts: &'a KeyboardShortcutsState,
        keybinding_diagnostics: &'a [String],
        theme_scheme: zeta_theme::ColorScheme,
        theme_follows_system: bool,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            bounds,
            section,
            palette,
            workspace_context,
            workspace_surface,
            keybindings,
            keyboard_shortcuts,
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
        if self.section != SettingsPageSection::Keybindings || self.keyboard_shortcuts.is_visible()
        {
            return Vec::new();
        }
        let navigation = NavigationGroupId::new(SETTINGS_SECTION_CONTENT);
        keyboard_shortcut_rows(self.keybindings)
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let value = row
                    .keybinding
                    .map(|keybinding| format_key_sequence(keybinding, self.keybindings.platform()))
                    .unwrap_or_else(|| "Unassigned".to_owned());
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
                .with_value(value)
            })
            .collect()
    }

    fn paint_section(&self, scene: &mut UiScene) {
        scene.with_clip(self.bounds, |scene| match self.section {
            SettingsPageSection::General => self.paint_general(scene),
            SettingsPageSection::LanguageServers => self.paint_placeholder(scene),
            SettingsPageSection::Appearance => self.paint_appearance(scene),
            SettingsPageSection::Keybindings => self.paint_keybindings(scene),
        });
    }

    fn paint_general(&self, scene: &mut UiScene) {
        self.paint_header(
            scene,
            "General",
            "Workspace and session defaults for this zeterm window.",
        );
        let rows = [
            (
                "Workspace",
                self.workspace_context.working_directory_label().to_owned(),
            ),
            (
                "Connection",
                self.workspace_context.location_label().to_owned(),
            ),
            ("Surface", surface_label(self.workspace_surface).to_owned()),
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
        let scheme = match self.theme_scheme {
            zeta_theme::ColorScheme::Dark | zeta_theme::ColorScheme::HighContrastDark => "Dark",
            zeta_theme::ColorScheme::Light | zeta_theme::ColorScheme::HighContrastLight => "Light",
        };
        let theme = if self.theme_follows_system {
            format!("System ({scheme})")
        } else {
            format!("Custom ({scheme})")
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
            PaintRect::new(swatch_bounds, self.palette.surface)
                .with_border(Border::uniform(1.0, self.palette.border))
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
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        );
        for (index, color) in [
            self.palette.background,
            self.palette.surface,
            self.palette.surface_raised,
            self.palette.accent,
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
                .with_border(Border::uniform(1.0, self.palette.border))
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
        let rows = keyboard_shortcut_rows(self.keybindings);
        for (index, row) in rows.iter().enumerate() {
            let bounds = self.keybinding_row_bounds(index);
            if self.dispatch.is_hovered(row.element)
                || self.dispatch.is_focused(row.element)
                || self.dispatch.is_pressed(row.element)
            {
                scene.draw_rect(
                    PaintRect::new(bounds, self.palette.surface_hovered)
                        .with_corner_radii(CornerRadii::uniform(4.0)),
                );
            }
            draw_label(
                scene,
                row.label,
                Rect::from_xywh(
                    bounds.origin.x + 10.0,
                    bounds.origin.y + 8.0,
                    bounds.size.width * 0.55,
                    20.0,
                ),
                TextStyle::new(13.0, self.palette.text).with_line_height(20.0),
            );
            let value = row
                .keybinding
                .map(|keybinding| format_key_sequence(keybinding, self.keybindings.platform()))
                .unwrap_or_else(|| "Unassigned".to_owned());
            draw_label(
                scene,
                &value,
                Rect::from_xywh(
                    bounds.origin.x + bounds.size.width * 0.58,
                    bounds.origin.y + 8.0,
                    bounds.size.width * 0.4,
                    20.0,
                ),
                TextStyle::new(12.0, self.palette.text_muted)
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
                TextStyle::new(12.0, self.palette.error).with_line_height(20.0),
            );
        }
    }

    fn paint_placeholder(&self, scene: &mut UiScene) {
        self.paint_header(scene, "Language Servers", "Language server preferences.");
    }

    fn paint_header(&self, scene: &mut UiScene, title: &str, description: &str) {
        let content = self.content_bounds();
        draw_label(
            scene,
            title,
            Rect::from_xywh(content.origin.x, content.origin.y, content.size.width, 30.0),
            TextStyle::new(22.0, self.palette.text)
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
            TextStyle::new(13.0, self.palette.text_muted).with_line_height(22.0),
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
            PaintRect::new(bounds, self.palette.surface)
                .with_border(Border::uniform(1.0, self.palette.border))
                .with_corner_radii(CornerRadii::uniform(6.0)),
        );
        for (index, (label, value)) in rows.iter().enumerate() {
            let row_y = bounds.origin.y + 12.0 + index as f32 * ROW_HEIGHT;
            draw_label(
                scene,
                label,
                Rect::from_xywh(bounds.origin.x + 14.0, row_y, 150.0, 20.0),
                TextStyle::new(12.0, self.palette.text_muted).with_line_height(20.0),
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
                TextStyle::new(13.0, self.palette.text).with_line_height(20.0),
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
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
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
            .with_parent(zeta_settings::SETTINGS_PAGE),
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
            SettingsPageSection::LanguageServers => "Language server settings",
            SettingsPageSection::Appearance => "Appearance settings",
            SettingsPageSection::Keybindings => "Keybinding settings",
        }
    }
}

fn card_height(row_count: usize) -> f32 {
    24.0 + row_count as f32 * ROW_HEIGHT
}

fn surface_label(surface: WorkspaceSurfaceKind) -> &'static str {
    match surface {
        WorkspaceSurfaceKind::Agent => "Agent workspace",
        WorkspaceSurfaceKind::Editor => "Editor",
        WorkspaceSurfaceKind::Terminal => "Terminal",
    }
}

fn draw_label(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(
        text.to_owned(),
        bounds.origin,
        bounds.size,
        style,
    ));
}

#[cfg(test)]
#[path = "settings_sections_tests.rs"]
mod tests;
