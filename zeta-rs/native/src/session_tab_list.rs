use zeta_icons::icons;
use zeta_ui::{
    Border, Component, FontFamily, FontWeight, PaintIcon, PaintRect, Rect, TextBlock, TextStyle,
    UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, InteractionFrame,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{ACTIVE_SESSION_TAB, SESSION_SIDEBAR, SESSION_TAB_LIST};
use crate::shell_style::ShellPalette;

const SIDEBAR_PADDING: f32 = 10.0;
const HEADER_HEIGHT: f32 = 28.0;
const TAB_HEIGHT: f32 = 62.0;
const TAB_ICON_SIZE: f32 = 16.0;

/// Product-owned vertical TabList for real terminal sessions.
pub(crate) struct SessionTabList<'a> {
    bounds: Rect,
    tab_bounds: Rect,
    title: &'a str,
    working_directory: &'a str,
    git_branch: &'a str,
    palette: ShellPalette,
    dispatch: &'a UiDispatch,
}

impl<'a> SessionTabList<'a> {
    pub(crate) fn new(
        bounds: Rect,
        title: &'a str,
        working_directory: &'a str,
        git_branch: &'a str,
        palette: ShellPalette,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let tab_bounds = Rect::from_xywh(
            bounds.origin.x + SIDEBAR_PADDING,
            bounds.origin.y + SIDEBAR_PADDING + HEADER_HEIGHT,
            (bounds.size.width - SIDEBAR_PADDING * 2.0).max(1.0),
            TAB_HEIGHT,
        );
        Self {
            bounds,
            tab_bounds,
            title,
            working_directory,
            git_branch,
            palette,
            dispatch,
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                SESSION_TAB_LIST,
                self.bounds,
                AccessibilityRole::TabList,
                "Terminal sessions",
            )
            .with_parent(SESSION_SIDEBAR),
        );
        frame.register(
            UiNode::new(
                ACTIVE_SESSION_TAB,
                self.tab_bounds,
                AccessibilityRole::Tab,
                format!(
                    "{}, {}, Git branch {}",
                    self.title, self.working_directory, self.git_branch
                ),
            )
            .with_parent(SESSION_TAB_LIST)
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate)
            .with_navigation(
                NavigationGroupId::new(SESSION_TAB_LIST),
                NavigationAxis::Vertical,
            )
            .with_selection(AccessibilitySelection::Selected),
        );
    }

    #[cfg(test)]
    pub(crate) const fn tab_bounds(&self) -> Rect {
        self.tab_bounds
    }
}

impl Component for SessionTabList<'_> {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_text(TextBlock::new(
            "SESSIONS",
            zeta_ui::Point::new(
                self.bounds.origin.x + SIDEBAR_PADDING + 4.0,
                self.bounds.origin.y + SIDEBAR_PADDING + 5.0,
            ),
            zeta_ui::Size::new(
                (self.bounds.size.width - SIDEBAR_PADDING * 2.0 - 8.0).max(1.0),
                18.0,
            ),
            TextStyle::new(11.0, self.palette.text_muted).with_weight(FontWeight::Bold),
        ));
        let fill = if self.dispatch.is_pressed(ACTIVE_SESSION_TAB) {
            self.palette.border
        } else if self.dispatch.is_hovered(ACTIVE_SESSION_TAB)
            || self.dispatch.is_focused(ACTIVE_SESSION_TAB)
        {
            self.palette.surface_hovered
        } else {
            self.palette.surface
        };
        scene.draw_rect(
            PaintRect::new(self.tab_bounds, fill)
                .with_border(Border::uniform(1.0, self.palette.border)),
        );
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(
                self.tab_bounds.origin.x,
                self.tab_bounds.origin.y,
                2.0,
                self.tab_bounds.size.height,
            ),
            self.palette.accent,
        ));
        scene.draw_icon(PaintIcon::new(
            icons::LOCAL,
            Rect::from_xywh(
                self.tab_bounds.origin.x + 12.0,
                self.tab_bounds.origin.y + 10.0,
                TAB_ICON_SIZE,
                TAB_ICON_SIZE,
            ),
            self.palette.accent,
        ));
        let text_x = self.tab_bounds.origin.x + 38.0;
        let text_width = (self.tab_bounds.right() - text_x - 10.0).max(1.0);
        scene.draw_text(TextBlock::new(
            self.title,
            zeta_ui::Point::new(text_x, self.tab_bounds.origin.y + 6.0),
            zeta_ui::Size::new(text_width, 18.0),
            TextStyle::new(13.0, self.palette.text).with_weight(FontWeight::Bold),
        ));
        let metadata_style = TextStyle::new(11.0, self.palette.text_muted)
            .with_family(FontFamily::Monospace)
            .with_line_height(15.0);
        scene.draw_text(TextBlock::new(
            self.working_directory,
            zeta_ui::Point::new(text_x, self.tab_bounds.origin.y + 24.0),
            zeta_ui::Size::new(text_width, 15.0),
            metadata_style.clone(),
        ));
        scene.draw_text(TextBlock::new(
            format!("git:({})", self.git_branch),
            zeta_ui::Point::new(text_x, self.tab_bounds.origin.y + 42.0),
            zeta_ui::Size::new(text_width, 15.0),
            metadata_style,
        ));
    }
}

#[cfg(test)]
#[path = "session_tab_list_tests.rs"]
mod tests;
