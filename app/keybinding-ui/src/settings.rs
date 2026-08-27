use zeta_ui::Border;
use zeta_ui::BoxShadow;
use zeta_ui::Color;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::CornerRadii;
use zeta_ui::Element;
use zeta_ui::InteractionRegion;
use zeta_ui::KeycapSequence;
use zeta_ui::KeycapStyle;
use zeta_ui::PaintRect;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::Size;
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

use crate::recording::KeyboardShortcutsState;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::KeySequence;
use zeta_keybinding::format_key_sequence;
use zeta_keybinding::keycap_labels;

const PANEL_WIDTH: f32 = 660.0;
const PANEL_HEIGHT: f32 = 470.0;
const PANEL_MARGIN: f32 = 24.0;
const HEADER_HEIGHT: f32 = 58.0;
const ROW_HEIGHT: f32 = 34.0;

/// Stable host-allocated identities used by the shortcut settings surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardShortcutsIds {
    parent: ElementId,
    root: ElementId,
    close: ElementId,
}

impl KeyboardShortcutsIds {
    pub const fn new(parent: ElementId, root: ElementId, close: ElementId) -> Self {
        Self {
            parent,
            root,
            close,
        }
    }
}

/// One host-owned command projected into the shortcut settings surface.
#[derive(Clone, Copy)]
pub struct KeyboardShortcutRow<'a, Command> {
    pub command: Command,
    pub element: ElementId,
    pub label: &'a str,
    pub keybinding: Option<&'a KeySequence>,
}

impl<'a, Command> KeyboardShortcutRow<'a, Command> {
    pub const fn new(
        command: Command,
        element: ElementId,
        label: &'a str,
        keybinding: Option<&'a KeySequence>,
    ) -> Self {
        Self {
            command,
            element,
            label,
            keybinding,
        }
    }
}

/// Visual tokens owned by the reusable shortcut settings component.
#[derive(Clone, Copy)]
pub struct KeyboardShortcutsStyle {
    surface: Color,
    border: Color,
    text: Color,
    text_muted: Color,
    surface_hovered: Color,
    selected: Color,
    close_hovered: Color,
}

impl KeyboardShortcutsStyle {
    pub const fn light() -> Self {
        Self {
            surface: Color::WHITE,
            border: Color::rgb(222, 222, 224),
            text: Color::rgb(38, 38, 41),
            text_muted: Color::rgb(126, 126, 132),
            surface_hovered: Color::rgb(248, 248, 249),
            selected: Color::rgb(232, 241, 239),
            close_hovered: Color::rgb(235, 235, 237),
        }
    }
}

impl Default for KeyboardShortcutsStyle {
    fn default() -> Self {
        Self::light()
    }
}

/// Modal shortcut settings presentation built from host-owned command rows.
pub struct KeyboardShortcuts<'a, Command> {
    viewport: Rect,
    panel: Rect,
    state: &'a KeyboardShortcutsState<Command>,
    rows: &'a [KeyboardShortcutRow<'a, Command>],
    diagnostics: &'a [String],
    ids: KeyboardShortcutsIds,
    style: KeyboardShortcutsStyle,
    platform: HostPlatform,
    dispatch: &'a UiDispatch,
}

impl<'a, Command: Copy + Eq> KeyboardShortcuts<'a, Command> {
    pub fn new(
        viewport: Rect,
        state: &'a KeyboardShortcutsState<Command>,
        rows: &'a [KeyboardShortcutRow<'a, Command>],
        diagnostics: &'a [String],
        ids: KeyboardShortcutsIds,
        platform: HostPlatform,
        dispatch: &'a UiDispatch,
    ) -> Option<Self> {
        if !state.is_visible() {
            return None;
        }
        let width = PANEL_WIDTH.min((viewport.size.width - PANEL_MARGIN * 2.0).max(1.0));
        let height = PANEL_HEIGHT.min((viewport.size.height - PANEL_MARGIN * 2.0).max(1.0));
        let panel = Rect::from_xywh(
            viewport.origin.x + (viewport.size.width - width) * 0.5,
            viewport.origin.y + (viewport.size.height - height) * 0.5,
            width,
            height,
        );
        Some(Self {
            viewport,
            panel,
            state,
            rows,
            diagnostics,
            ids,
            style: KeyboardShortcutsStyle::default(),
            platform,
            dispatch,
        })
    }

    pub const fn with_style(mut self, style: KeyboardShortcutsStyle) -> Self {
        self.style = style;
        self
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let mut regions = Vec::with_capacity(self.rows.len() + 1);
        let close = self.close_bounds();
        regions.push(
            InteractionRegion::new(
                "KeyboardShortcutsClose",
                self.ids.close,
                close,
                AccessibilityRole::Button,
                "Close keyboard shortcuts",
            )
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate),
        );
        let navigation = NavigationGroupId::new(self.ids.root);
        for (index, row) in self.rows.iter().enumerate() {
            let value = row
                .keybinding
                .map(|keybinding| format_key_sequence(keybinding, self.platform))
                .unwrap_or_else(|| "Unassigned".to_owned());
            regions.push(
                InteractionRegion::new(
                    "KeyboardShortcutRow",
                    row.element,
                    self.row_bounds(index),
                    AccessibilityRole::Button,
                    format!("Record shortcut for {}", row.label),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Vertical)
                .with_value(value),
            );
        }
        regions
    }

    fn close_bounds(&self) -> Rect {
        Rect::from_xywh(
            self.panel.right() - 42.0,
            self.panel.origin.y + 14.0,
            28.0,
            28.0,
        )
    }

    fn row_bounds(&self, index: usize) -> Rect {
        Rect::from_xywh(
            self.panel.origin.x + 16.0,
            self.panel.origin.y + HEADER_HEIGHT + index as f32 * ROW_HEIGHT,
            self.panel.size.width - 32.0,
            ROW_HEIGHT,
        )
    }

    fn paint_row(&self, scene: &mut UiScene, index: usize, row: &KeyboardShortcutRow<'_, Command>) {
        let bounds = self.row_bounds(index);
        let selected = self.state.recording_command() == Some(row.command);
        if selected
            || self.dispatch.is_hovered(row.element)
            || self.dispatch.is_focused(row.element)
            || self.dispatch.is_pressed(row.element)
        {
            let fill = if selected {
                self.style.selected
            } else {
                self.style.surface_hovered
            };
            scene.draw_rect(
                PaintRect::new(bounds, fill).with_corner_radii(CornerRadii::uniform(4.0)),
            );
        }
        draw_label(
            scene,
            row.label,
            Point::new(bounds.origin.x + 10.0, bounds.origin.y + 8.0),
            Size::new((bounds.size.width * 0.5).max(1.0), 18.0),
            TextStyle::new(13.0, self.style.text).with_line_height(18.0),
        );
        let recorded = selected.then(|| self.state.recorded_keybinding()).flatten();
        let keybinding = recorded.as_ref().or(row.keybinding);
        let labels = keybinding
            .map(|keybinding| keycap_labels(keybinding, self.platform))
            .unwrap_or_default();
        if labels.is_empty() {
            draw_label(
                scene,
                if selected {
                    "Press keys…"
                } else {
                    "Unassigned"
                },
                Point::new(bounds.right() - 130.0, bounds.origin.y + 8.0),
                Size::new(120.0, 18.0),
                TextStyle::new(12.0, self.style.text_muted).with_line_height(18.0),
            );
            return;
        }
        let style = keycap_style();
        let measured = KeycapSequence::new(Point::new(0.0, 0.0), labels.clone(), style.clone());
        let sequence = KeycapSequence::new(
            Point::new(
                (bounds.right() - measured.bounds().size.width - 10.0)
                    .max(bounds.origin.x + bounds.size.width * 0.5),
                bounds.origin.y + (bounds.size.height - measured.bounds().size.height) * 0.5,
            ),
            labels,
            style,
        );
        scene.draw_component(&sequence);
    }

    fn paint_footer(&self, scene: &mut UiScene) {
        let message = self
            .state
            .status_message()
            .or_else(|| {
                self.diagnostics
                    .first()
                    .map(|diagnostic| (diagnostic.as_str(), true))
            })
            .unwrap_or(("Select a command, then press up to four key chords.", false));
        let color = if message.1 {
            Color::rgb(176, 54, 64)
        } else {
            self.style.text_muted
        };
        draw_label(
            scene,
            message.0,
            Point::new(self.panel.origin.x + 20.0, self.panel.bottom() - 32.0),
            Size::new(self.panel.size.width - 40.0, 18.0),
            TextStyle::new(12.0, color).with_line_height(18.0),
        );
    }
}

impl<Command: Copy + Eq> Component for KeyboardShortcuts<'_, Command> {
    fn element(&self) -> ComponentElement {
        Element::leaf("KeyboardShortcuts")
            .corner_radii(CornerRadii::uniform(8.0))
            .in_overlay(self.panel)
            .with_identity(self.ids.root)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                self.ids.root,
                element.bounds(),
                AccessibilityRole::Group,
                "Keyboard shortcuts",
            )
            .with_parent(self.ids.parent),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(self.ids.root);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        self.paint_contents(context.scene_mut());
    }

    fn paint_element(&self, scene: &mut UiScene, _element: &ComputedElement) {
        self.paint_contents(scene);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.with_element(self.element(), |scene, _element| self.paint_contents(scene));
    }
}

impl<Command: Copy + Eq> KeyboardShortcuts<'_, Command> {
    fn paint_contents(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.viewport, Color::rgba(20, 20, 24, 72)));
        scene.draw_rect(
            PaintRect::new(self.panel, self.style.surface)
                .with_shadow(
                    BoxShadow::new(Color::rgba(0, 0, 0, 64))
                        .with_offset(Point::new(0.0, 8.0))
                        .with_blur_radius(24.0),
                )
                .with_border(Border::uniform(1.0, self.style.border))
                .with_corner_radii(CornerRadii::uniform(8.0)),
        );
        draw_label(
            scene,
            "Keyboard shortcuts",
            Point::new(self.panel.origin.x + 20.0, self.panel.origin.y + 18.0),
            Size::new(self.panel.size.width - 80.0, 24.0),
            TextStyle::new(17.0, self.style.text).with_line_height(22.0),
        );
        let close = self.close_bounds();
        if self.dispatch.is_hovered(self.ids.close) || self.dispatch.is_focused(self.ids.close) {
            scene.draw_rect(
                PaintRect::new(close, self.style.close_hovered)
                    .with_corner_radii(CornerRadii::uniform(4.0)),
            );
        }
        draw_label(
            scene,
            "×",
            close.origin,
            close.size,
            TextStyle::new(18.0, self.style.text_muted).with_line_height(24.0),
        );
        for (index, row) in self.rows.iter().enumerate() {
            self.paint_row(scene, index, row);
        }
        self.paint_footer(scene);
    }
}

pub fn paint_chord_hint(
    scene: &mut UiScene,
    viewport: Rect,
    keybinding: &KeySequence,
    entered: usize,
    platform: HostPlatform,
) {
    let labels = keycap_labels(keybinding, platform)
        .into_iter()
        .take(entered)
        .collect::<Vec<_>>();
    let measured = KeycapSequence::new(Point::new(0.0, 0.0), labels.clone(), keycap_style());
    let width = measured.bounds().size.width + 150.0;
    let bounds = Rect::from_xywh(
        viewport.origin.x + (viewport.size.width - width) * 0.5,
        viewport.bottom() - 54.0,
        width,
        36.0,
    );
    scene.with_overlay(|scene| {
        scene.draw_rect(
            PaintRect::new(bounds, Color::rgb(45, 46, 51))
                .with_shadow(
                    BoxShadow::new(Color::rgba(0, 0, 0, 48))
                        .with_offset(Point::new(0.0, 4.0))
                        .with_blur_radius(12.0),
                )
                .with_corner_radii(CornerRadii::uniform(6.0)),
        );
        let sequence = KeycapSequence::new(
            Point::new(bounds.origin.x + 8.0, bounds.origin.y + 7.0),
            labels,
            keycap_style(),
        );
        scene.draw_component(&sequence);
        draw_label(
            scene,
            "waiting for next key…",
            Point::new(sequence.bounds().right() + 10.0, bounds.origin.y + 9.0),
            Size::new(132.0, 18.0),
            TextStyle::new(12.0, Color::rgb(220, 220, 224)).with_line_height(18.0),
        );
    });
}

fn keycap_style() -> KeycapStyle {
    KeycapStyle::new(Color::rgb(62, 63, 69), Color::WHITE)
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_height(22.0)
        .with_minimum_width(22.0)
        .with_horizontal_padding(6.0)
}

fn draw_label(scene: &mut UiScene, label: &str, origin: Point, size: Size, style: TextStyle) {
    scene.draw_text(TextBlock::new(label.to_owned(), origin, size, style));
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
