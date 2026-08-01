use zeta_ui::{Border, Color, CornerRadii, InspectionNode, PaintRect, Point, Rect, UiScene};
use zeta_winit::{ElementState, Key, KeyEvent, LogicalSize, ModifiersState, MouseButton, NamedKey};

use crate::NativeApp;
use crate::shell_scene::LogicalViewport;

mod panel;

pub(crate) const PANEL_WIDTH: f32 = 360.0;
const OUTLINE_COLOR: Color = Color::rgb(35, 131, 226);
const ANCESTOR_OUTLINE_COLOR: Color = Color::rgba(116, 92, 217, 150);
const PADDING_COLOR: Color = Color::rgba(238, 147, 54, 92);

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InspectionSelection {
    pub(super) path: Vec<InspectionNode>,
}

impl InspectionSelection {
    fn target(&self) -> Option<&InspectionNode> {
        self.path.last()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LayoutInspector {
    enabled: bool,
    picking: bool,
    content_width: Option<f32>,
    hovered: Option<InspectionSelection>,
    locked: Option<InspectionSelection>,
}

impl LayoutInspector {
    pub(crate) const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) const fn is_picking(&self) -> bool {
        self.enabled && self.picking
    }

    pub(crate) fn content_viewport(&self, window_viewport: LogicalViewport) -> LogicalViewport {
        LogicalViewport {
            width: self.content_width.unwrap_or(window_viewport.width),
            height: window_viewport.height,
        }
    }

    pub(crate) fn uses_inspection_cursor(&self, pointer: Option<Point>) -> bool {
        self.is_picking()
            && pointer.is_some_and(|point| point.x < self.content_width.unwrap_or(f32::INFINITY))
    }

    pub(crate) fn pointer_is_over_panel(&self, pointer: Option<Point>) -> bool {
        self.enabled
            && pointer.is_some_and(|point| point.x >= self.content_width.unwrap_or(f32::INFINITY))
    }

    pub(crate) fn window_resized(&mut self, window_viewport: LogicalViewport) {
        let Some(content_width) = self.content_width else {
            return;
        };
        if !self.enabled && window_viewport.width <= content_width + 0.5 {
            self.content_width = None;
        }
    }

    fn open(&mut self, content_width: f32) {
        self.enabled = true;
        self.picking = false;
        self.content_width = Some(content_width);
        self.hovered = None;
        self.locked = None;
    }

    fn close(&mut self) -> Option<f32> {
        self.enabled = false;
        self.picking = false;
        self.hovered = None;
        self.locked = None;
        self.content_width
    }

    fn stop_picking_or_close(&mut self) -> Option<f32> {
        if self.picking {
            self.picking = false;
            None
        } else {
            self.close()
        }
    }

    fn toggle_picking(&mut self) {
        if self.picking {
            self.picking = false;
        } else {
            self.picking = true;
            self.locked = None;
        }
    }

    fn select(&mut self, target: Option<InspectionSelection>) {
        self.locked = target;
        self.picking = false;
    }

    pub(crate) fn decorate(
        &mut self,
        scene: &mut UiScene,
        window_viewport: LogicalViewport,
        pointer: Option<Point>,
    ) {
        if !self.enabled {
            return;
        }
        let content_width = self.content_width.unwrap_or(window_viewport.width);
        if self.picking
            && self.locked.is_none()
            && let Some(point) = pointer.filter(|point| point.x < content_width)
        {
            self.hovered = selection_at(scene, point);
        }
        let selection = self.locked.as_ref().or(self.hovered.as_ref());
        scene.with_overlay(|scene| {
            if let Some(selection) = selection {
                paint_selection(scene, selection);
            }
            panel::paint(
                scene,
                window_viewport,
                content_width,
                selection,
                panel::PanelState {
                    picking: self.picking,
                    picker_hovered: pointer
                        .is_some_and(|point| panel::picker_bounds(content_width).contains(point)),
                    has_selection: selection.is_some(),
                },
            );
        });
    }
}

impl NativeApp {
    pub(super) fn route_layout_inspector_keyboard(&mut self, event: &KeyEvent) -> bool {
        if is_toggle_shortcut(event, self.modifiers) {
            if self.layout_inspector.is_enabled() {
                if let Some(content_width) = self.layout_inspector.close() {
                    self.request_layout_inspector_window_width(content_width);
                }
            } else {
                let content_width = self.window_viewport().width;
                self.layout_inspector.open(content_width);
                self.request_layout_inspector_window_width(content_width + PANEL_WIDTH);
                let _ = self.ui_dispatch.pointer_left();
            }
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
            return true;
        }
        if self.layout_inspector.is_enabled() && event.logical_key == Key::Named(NamedKey::Escape) {
            if let Some(content_width) = self.layout_inspector.stop_picking_or_close() {
                self.request_layout_inspector_window_width(content_width);
            }
            self.rebuild_presentation();
            self.update_cursor();
            self.request_redraw();
            return true;
        }
        self.layout_inspector.is_picking()
    }

    pub(super) fn route_layout_inspector_pointer_move(&mut self) -> bool {
        if !self.layout_inspector.is_enabled() {
            return false;
        }
        self.rebuild_presentation_on_next_redraw();
        self.update_cursor();
        self.layout_inspector.is_picking()
            || self
                .layout_inspector
                .pointer_is_over_panel(self.cursor_position)
    }

    pub(super) fn route_layout_inspector_pointer_left(&mut self) -> bool {
        if !self.layout_inspector.is_enabled() {
            return false;
        }
        self.rebuild_presentation_on_next_redraw();
        self.update_cursor();
        self.layout_inspector.is_picking()
    }

    pub(super) fn route_layout_inspector_button(
        &mut self,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        if !self.layout_inspector.is_enabled() {
            return false;
        }
        let content_width = self.logical_viewport().width;
        if state == ElementState::Pressed
            && button == MouseButton::Left
            && self
                .cursor_position
                .is_some_and(|point| panel::picker_bounds(content_width).contains(point))
        {
            self.layout_inspector.toggle_picking();
            self.rebuild_presentation_on_next_redraw();
            self.update_cursor();
            return true;
        }
        if self
            .layout_inspector
            .pointer_is_over_panel(self.cursor_position)
        {
            return true;
        }
        if !self.layout_inspector.is_picking() {
            return false;
        }
        if state == ElementState::Released && button == MouseButton::Left {
            let target = self.presentation.as_ref().and_then(|presentation| {
                let point = self
                    .cursor_position
                    .filter(|point| point.x < content_width)?;
                selection_at(&presentation.scene, point)
            });
            self.layout_inspector.select(target);
            self.rebuild_presentation_on_next_redraw();
            self.update_cursor();
        }
        true
    }

    fn request_layout_inspector_window_width(&self, width: f32) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        window.request_inner_logical_size(LogicalSize::new(
            width as f64,
            self.window_viewport().height as f64,
        ));
    }
}

fn selection_at(scene: &UiScene, point: Point) -> Option<InspectionSelection> {
    let target = scene.inspection().target_at(point)?;
    Some(InspectionSelection {
        path: scene
            .inspection()
            .ancestry(target.id())
            .into_iter()
            .cloned()
            .collect(),
    })
}

fn is_toggle_shortcut(event: &KeyEvent, modifiers: ModifiersState) -> bool {
    let Key::Character(character) = &event.logical_key else {
        return false;
    };
    let primary = if cfg!(target_os = "macos") {
        modifiers.super_key()
    } else {
        modifiers.control_key()
    };
    primary
        && modifiers.shift_key()
        && !modifiers.alt_key()
        && character.as_str().eq_ignore_ascii_case("i")
}

fn paint_selection(scene: &mut UiScene, selection: &InspectionSelection) {
    let Some(target) = selection.target() else {
        return;
    };
    paint_padding(scene, target);
    for node in &selection.path {
        let selected = node.id() == target.id();
        scene.draw_rect(
            PaintRect::new(node.bounds(), Color::TRANSPARENT)
                .with_border(Border::uniform(
                    if selected { 2.0 } else { 1.0 },
                    if selected {
                        OUTLINE_COLOR
                    } else {
                        ANCESTOR_OUTLINE_COLOR
                    },
                ))
                .with_corner_radii(node.corner_radii().unwrap_or(CornerRadii::uniform(0.0))),
        );
    }
}

fn paint_padding(scene: &mut UiScene, node: &InspectionNode) {
    let Some(padding) = node.padding() else {
        return;
    };
    let bounds = node.bounds();
    let top = padding.top.max(0.0).min(bounds.size.height);
    let bottom = padding
        .bottom
        .max(0.0)
        .min((bounds.size.height - top).max(0.0));
    let middle_height = (bounds.size.height - top - bottom).max(0.0);
    let left = padding.left.max(0.0).min(bounds.size.width);
    let right = padding
        .right
        .max(0.0)
        .min((bounds.size.width - left).max(0.0));
    for padding_bounds in [
        Rect::from_xywh(bounds.origin.x, bounds.origin.y, bounds.size.width, top),
        Rect::from_xywh(
            bounds.origin.x,
            bounds.bottom() - bottom,
            bounds.size.width,
            bottom,
        ),
        Rect::from_xywh(bounds.origin.x, bounds.origin.y + top, left, middle_height),
        Rect::from_xywh(
            bounds.right() - right,
            bounds.origin.y + top,
            right,
            middle_height,
        ),
    ] {
        if !padding_bounds.is_empty() {
            scene.draw_rect(PaintRect::new(padding_bounds, PADDING_COLOR));
        }
    }
}

#[cfg(test)]
#[path = "layout_inspector_tests.rs"]
mod tests;
