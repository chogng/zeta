use zeta_ui::Border;
use zeta_ui::Button;
use zeta_ui::ButtonState;
use zeta_ui::CaretVisibility;
use zeta_ui::Color;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::CornerRadii;
use zeta_ui::Element;
use zeta_ui::FontWeight;
use zeta_ui::InputBox;
use zeta_ui::InputBoxState;
use zeta_ui::InteractionRegion;
use zeta_ui::ListView;
use zeta_ui::PaintRect;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::ScrollMetrics;
use zeta_ui::Size;
use zeta_ui::TextBlock;
use zeta_ui::TextInputLayoutEngine;
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

use crate::remote_tunnel_manager::REMOTE_TUNNEL_ITEM_HEIGHT;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_LIST;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_MANAGER;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_MANAGER_CLOSE;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_OPEN;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_REMOTE_PORT;
use crate::remote_tunnel_manager::REMOTE_TUNNEL_STATUS;
use crate::remote_tunnel_manager::RemoteTunnelLifecycle;
use crate::remote_tunnel_manager::RemoteTunnelManagerState;
use crate::remote_tunnel_manager::remote_tunnel_item_id;
use crate::remote_tunnel_manager::remote_tunnel_stop_id;
use crate::shell_interaction::WINDOW;
use crate::shell_style::ShellPalette;

const PANEL_WIDTH: f32 = 640.0;
const PANEL_HEIGHT: f32 = 430.0;
const PANEL_MARGIN: f32 = 20.0;
const CONTENT_INSET: f32 = 24.0;
const INPUT_HEIGHT: f32 = 36.0;
const BUTTON_HEIGHT: f32 = 34.0;

#[path = "remote_tunnel_manager_style.rs"]
mod style;
use style::action_button_style;
use style::input_style;

pub(crate) struct RemoteTunnelManager<'a> {
    viewport: Rect,
    panel: Rect,
    state: &'a RemoteTunnelManagerState,
    palette: ShellPalette,
    dispatch: &'a UiDispatch,
    list: ListView,
    remote_port: InputBox,
}

impl<'a> RemoteTunnelManager<'a> {
    pub(crate) fn new(
        viewport: Rect,
        state: &'a RemoteTunnelManagerState,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &'a UiDispatch,
    ) -> Option<Self> {
        if !state.is_open() {
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
        let list = ListView::new(
            list_bounds(panel),
            state.tunnels().len(),
            REMOTE_TUNNEL_ITEM_HEIGHT,
            state.scroll_state(),
            palette.file_list_scroll_view_style(),
        );
        let remote_port = InputBox::new(
            remote_port_bounds(panel),
            "3000",
            input_state(dispatch, caret_visibility),
            input_style(palette),
            state.remote_port_input(),
            text_layout,
        );
        Some(Self {
            viewport,
            panel,
            state,
            palette,
            dispatch,
            list,
            remote_port,
        })
    }

    pub(crate) const fn remote_port_caret_bounds(&self) -> Option<Rect> {
        self.remote_port.caret_bounds()
    }

    pub(crate) fn list_scroll_metrics(&self) -> ScrollMetrics {
        self.list.scroll_view().metrics()
    }

    pub(crate) const fn list_viewport_bounds(&self) -> Rect {
        self.list.scroll_view().bounds()
    }

    #[cfg(test)]
    pub(crate) const fn panel_bounds(&self) -> Rect {
        self.panel
    }

    fn interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation = NavigationGroupId::new(REMOTE_TUNNEL_MANAGER);
        let mut regions = vec![
            InteractionRegion::new(
                "RemoteTunnelList",
                REMOTE_TUNNEL_LIST,
                self.list_viewport_bounds(),
                AccessibilityRole::List,
                "Active SSH tunnels",
            )
            .with_parent(REMOTE_TUNNEL_MANAGER),
            button_region(
                REMOTE_TUNNEL_MANAGER_CLOSE,
                close_bounds(self.panel),
                "Close Remote tunnel manager; active tunnels remain open",
                navigation,
                true,
            ),
            InteractionRegion::new(
                "RemoteTunnelPortInput",
                REMOTE_TUNNEL_REMOTE_PORT,
                self.remote_port.bounds(),
                AccessibilityRole::TextInput,
                "Remote loopback TCP port",
            )
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation, NavigationAxis::Vertical)
            .with_value(self.state.remote_port_input().text()),
            button_region(
                REMOTE_TUNNEL_OPEN,
                open_bounds(self.panel),
                "Open SSH tunnel",
                navigation,
                true,
            ),
        ];
        for index in self.list.visible_range() {
            let Some(bounds) = self.list.item_bounds(index) else {
                continue;
            };
            let tunnel = &self.state.tunnels()[index];
            let row = bounds.intersection(self.list_viewport_bounds());
            regions.push(
                InteractionRegion::new(
                    "RemoteTunnelItem",
                    remote_tunnel_item_id(tunnel.tunnel_id()),
                    row,
                    AccessibilityRole::ListItem,
                    tunnel_label(tunnel),
                )
                .with_parent(REMOTE_TUNNEL_LIST),
            );
            regions.push(button_region(
                remote_tunnel_stop_id(tunnel.tunnel_id()),
                stop_bounds(bounds).intersection(self.list_viewport_bounds()),
                if tunnel.lifecycle() == RemoteTunnelLifecycle::Stopping {
                    "SSH tunnel is stopping"
                } else {
                    "Stop SSH tunnel"
                },
                navigation,
                self.state.can_stop(tunnel.tunnel_id()),
            ));
        }
        if let Some((status, _)) = self.state.status() {
            regions.push(
                InteractionRegion::new(
                    "RemoteTunnelStatus",
                    REMOTE_TUNNEL_STATUS,
                    status_bounds(self.panel),
                    AccessibilityRole::Group,
                    status,
                )
                .with_parent(REMOTE_TUNNEL_MANAGER),
            );
        }
        regions
    }

    fn button_state(&self, id: ElementId, enabled: bool) -> ButtonState {
        if !enabled {
            ButtonState::Disabled
        } else if self.dispatch.is_pressed(id) {
            ButtonState::Pressed
        } else if self.dispatch.is_focused(id) {
            ButtonState::Focused
        } else if self.dispatch.is_hovered(id) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        }
    }

    fn paint_content(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.viewport, Color::rgba(0, 0, 0, 76)));
        scene.draw_rect(
            PaintRect::new(self.panel, self.palette.surface)
                .with_border(Border::uniform(1.0, self.palette.border))
                .with_corner_radii(CornerRadii::uniform(8.0)),
        );
        draw_text(
            scene,
            "Remote Tunnels",
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 18.0,
                self.panel.size.width - CONTENT_INSET * 2.0 - 40.0,
                26.0,
            ),
            TextStyle::new(18.0, self.palette.text)
                .with_line_height(26.0)
                .with_weight(FontWeight::Bold),
        );
        draw_text(
            scene,
            &format!(
                "SSH host: {} · binds locally to 127.0.0.1 only",
                self.state.host()
            ),
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 48.0,
                self.panel.size.width - CONTENT_INSET * 2.0,
                18.0,
            ),
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        );
        draw_text(
            scene,
            "Remote port",
            Rect::from_xywh(
                remote_port_bounds(self.panel).origin.x,
                remote_port_bounds(self.panel).origin.y - 21.0,
                140.0,
                18.0,
            ),
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        );
        scene.draw_component(&self.remote_port);
        scene.draw_component(&Button::new(
            open_bounds(self.panel),
            "Open Tunnel",
            self.button_state(REMOTE_TUNNEL_OPEN, true),
            action_button_style(self.palette, true),
        ));
        scene.draw_rect(
            PaintRect::new(list_bounds(self.panel), self.palette.surface_raised)
                .with_border(Border::uniform(1.0, self.palette.border))
                .with_corner_radii(CornerRadii::uniform(4.0)),
        );
        if self.state.tunnels().is_empty() {
            draw_text(
                scene,
                "No active tunnels. Closing this panel does not close a running tunnel.",
                Rect::from_xywh(
                    list_bounds(self.panel).origin.x + 12.0,
                    list_bounds(self.panel).origin.y + 12.0,
                    list_bounds(self.panel).size.width - 24.0,
                    20.0,
                ),
                TextStyle::new(12.0, self.palette.text_muted).with_line_height(20.0),
            );
        } else {
            self.list.draw(scene, |scene, layout| {
                let tunnel = &self.state.tunnels()[layout.index()];
                draw_text(
                    scene,
                    &tunnel_label(tunnel),
                    Rect::from_xywh(
                        layout.bounds().origin.x + 12.0,
                        layout.bounds().origin.y + 11.0,
                        (layout.bounds().size.width - 112.0).max(1.0),
                        20.0,
                    ),
                    TextStyle::new(12.0, self.palette.text).with_line_height(20.0),
                );
                let stop_id = remote_tunnel_stop_id(tunnel.tunnel_id());
                scene.draw_component(&Button::new(
                    stop_bounds(layout.bounds()),
                    if tunnel.lifecycle() == RemoteTunnelLifecycle::Stopping {
                        "Stopping"
                    } else {
                        "Stop"
                    },
                    self.button_state(stop_id, self.state.can_stop(tunnel.tunnel_id())),
                    action_button_style(self.palette, false),
                ));
            });
        }
        if let Some((status, error)) = self.state.status() {
            draw_text(
                scene,
                status,
                status_bounds(self.panel),
                TextStyle::new(
                    12.0,
                    if error {
                        self.palette.error
                    } else {
                        self.palette.text_muted
                    },
                )
                .with_line_height(18.0),
            );
        }
        scene.draw_component(&Button::new(
            close_bounds(self.panel),
            "×",
            self.button_state(REMOTE_TUNNEL_MANAGER_CLOSE, true),
            action_button_style(self.palette, false),
        ));
    }
}

impl Component for RemoteTunnelManager<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("RemoteTunnelManager")
            .in_bounds(self.panel)
            .with_identity(REMOTE_TUNNEL_MANAGER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                REMOTE_TUNNEL_MANAGER,
                element.bounds(),
                AccessibilityRole::Group,
                "Remote tunnel manager",
            )
            .with_parent(WINDOW),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(REMOTE_TUNNEL_MANAGER);
        for region in self.interaction_regions() {
            context.draw_component(&region);
        }
        self.paint_content(context.scene_mut());
    }

    fn paint(&self, scene: &mut UiScene) {
        self.paint_content(scene);
    }
}

fn input_state(dispatch: &UiDispatch, caret_visibility: CaretVisibility) -> InputBoxState {
    if dispatch.is_focused(REMOTE_TUNNEL_REMOTE_PORT) {
        InputBoxState::Focused(caret_visibility)
    } else if dispatch.is_hovered(REMOTE_TUNNEL_REMOTE_PORT) {
        InputBoxState::Hovered
    } else {
        InputBoxState::Resting
    }
}

fn button_region(
    id: ElementId,
    bounds: Rect,
    label: &str,
    navigation: NavigationGroupId,
    enabled: bool,
) -> InteractionRegion {
    let region = InteractionRegion::new(
        "RemoteTunnelButton",
        id,
        bounds,
        AccessibilityRole::Button,
        label,
    )
    .with_cursor(CursorFeedback::Pointer)
    .with_focus(FocusBehavior::TabStop)
    .with_navigation(navigation, NavigationAxis::Vertical);
    if enabled {
        region.with_action(NodeAction::Activate)
    } else {
        region
    }
}

fn tunnel_label(tunnel: &crate::remote_tunnel_manager::RemoteTunnelRecord) -> String {
    let local = tunnel
        .local_port()
        .map(|port| port.to_string())
        .unwrap_or_else(|| "allocating".into());
    format!(
        "127.0.0.1:{local} → Remote 127.0.0.1:{} · {}",
        tunnel.remote_port(),
        tunnel.lifecycle().label()
    )
}

fn remote_port_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET,
        panel.origin.y + 98.0,
        180.0,
        INPUT_HEIGHT,
    )
}

fn open_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        remote_port_bounds(panel).right() + 12.0,
        remote_port_bounds(panel).origin.y + 1.0,
        112.0,
        BUTTON_HEIGHT,
    )
}

fn list_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET,
        panel.origin.y + 158.0,
        panel.size.width - CONTENT_INSET * 2.0,
        190.0,
    )
}

fn status_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(
        panel.origin.x + CONTENT_INSET,
        panel.origin.y + 362.0,
        panel.size.width - CONTENT_INSET * 2.0 - 50.0,
        40.0,
    )
}

fn close_bounds(panel: Rect) -> Rect {
    Rect::from_xywh(panel.right() - 48.0, panel.origin.y + 12.0, 32.0, 32.0)
}

fn stop_bounds(row: Rect) -> Rect {
    Rect::from_xywh(row.right() - 86.0, row.origin.y + 4.0, 76.0, 34.0)
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(
        text,
        Point::new(bounds.origin.x, bounds.origin.y),
        Size::new(bounds.size.width, bounds.size.height),
        style,
    ));
}

#[cfg(test)]
#[path = "remote_tunnel_manager_view_tests.rs"]
mod tests;
