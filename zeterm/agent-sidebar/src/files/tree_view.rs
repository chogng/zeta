use zeta_icons::Icon;
use zeta_icons::icons;
use zeta_ui::Color;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::Element;
use zeta_ui::PaintIcon;
use zeta_ui::PaintRect;
use zeta_ui::Rect;
use zeta_ui::TextBlock;
use zeta_ui::TextStyle;
use zeta_ui::TreeItemExpansion;
use zeta_ui::TreeItemLayout;
use zeta_ui::TreeView;
use zeta_ui::TreeViewStyle;
use zeta_ui::UiScene;
use zui::AccessibilityExpansion;
use zui::AccessibilityRole;
use zui::ComputedElement;
use zui::CursorFeedback;
use zui::ElementId;
use zui::FocusBehavior;
use zui::NavigationAxis;
use zui::NavigationGroupId;
use zui::NodeAction;
use zui::UiDispatch;

use super::FILE_LIST_ROW_HEIGHT;
use super::FilesState;
use super::pane::EXPLORER_PANE;
use super::pane::FilesPaneStyle;
use super::pane::HORIZONTAL_PADDING;
use super::pane::ICON_SIZE;
use super::pane::OVERSCAN_ITEMS;

pub(super) fn build_tree_view<'a>(
    bounds: Rect,
    files: &'a FilesState,
    style: &FilesPaneStyle,
) -> TreeView<'a> {
    TreeView::new(
        bounds,
        files.tree_items(),
        files.scroll_state(),
        TreeViewStyle::new(style.scroll_view, FILE_LIST_ROW_HEIGHT),
    )
    .with_overscan_items(OVERSCAN_ITEMS)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum FileRowState {
    Resting,
    Hovered,
    Selected,
}

pub(super) struct FilesTreeView<'a> {
    bounds: Rect,
    files: &'a FilesState,
    style: &'a FilesPaneStyle,
    selected: Option<ElementId>,
    focused: Option<ElementId>,
    dispatch: &'a UiDispatch,
}

impl<'a> FilesTreeView<'a> {
    pub(super) fn new(
        bounds: Rect,
        files: &'a FilesState,
        style: &'a FilesPaneStyle,
        selected: Option<ElementId>,
        focused: Option<ElementId>,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            bounds,
            files,
            style,
            selected,
            focused,
            dispatch,
        }
    }
}

impl Component for FilesTreeView<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesTree").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        let tree = build_tree_view(self.bounds, self.files, self.style);
        let visible_range = tree.visible_range();
        tree.draw(scene, |scene, layout| {
            let Some(row) = self.files.tree_row(layout.index()) else {
                return;
            };
            let element = row.entry().element_id();
            let selected = self.selected == Some(element);
            let focused = self.focused == Some(element);
            scene.draw_component(&FilesTreeItem::new(
                layout,
                element,
                row.entry().label(),
                self.style,
                selected,
                focused,
                self.dispatch.is_hovered(element),
                visible_range.contains(&layout.index()),
            ));
        });
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        let tree = build_tree_view(self.bounds, self.files, self.style);
        let visible_range = tree.visible_range();
        tree.draw_components(context, |context, layout| {
            let Some(row) = self.files.tree_row(layout.index()) else {
                return;
            };
            let element = row.entry().element_id();
            let selected = self.selected == Some(element);
            let focused = self.focused == Some(element);
            context.draw_component(&FilesTreeItem::new(
                layout,
                element,
                row.entry().label(),
                self.style,
                selected,
                focused,
                self.dispatch.is_hovered(element),
                visible_range.contains(&layout.index()),
            ));
        });
    }
}

struct FilesTreeItem<'a> {
    layout: TreeItemLayout,
    element: ElementId,
    label: &'a str,
    style: &'a FilesPaneStyle,
    state: FileRowState,
    selected: bool,
    interactive: bool,
}

impl<'a> FilesTreeItem<'a> {
    fn new(
        layout: TreeItemLayout,
        element: ElementId,
        label: &'a str,
        style: &'a FilesPaneStyle,
        selected: bool,
        focused: bool,
        hovered: bool,
        interactive: bool,
    ) -> Self {
        Self {
            layout,
            element,
            label,
            style,
            state: file_row_state(selected || focused, hovered),
            selected,
            interactive,
        }
    }
}

impl Component for FilesTreeItem<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesTreeItem")
            .in_bounds(self.layout.bounds())
            .with_identity(self.element)
            .with_inspection_label(self.label)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<zui::UiNode> {
        if !self.interactive {
            return None;
        }
        let expansion = match self.layout.item().expansion() {
            TreeItemExpansion::Leaf => AccessibilityExpansion::NotApplicable,
            TreeItemExpansion::Collapsed => AccessibilityExpansion::Collapsed,
            TreeItemExpansion::Expanded => AccessibilityExpansion::Expanded,
        };
        Some(
            zui::UiNode::new(
                self.element,
                element.bounds(),
                AccessibilityRole::TreeItem,
                self.label,
            )
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate)
            .with_navigation(
                NavigationGroupId::new(EXPLORER_PANE),
                NavigationAxis::Vertical,
            )
            .with_level(self.layout.item().depth() + 1)
            .with_expansion(expansion)
            .with_selection(if self.selected {
                zui::AccessibilitySelection::Selected
            } else {
                zui::AccessibilitySelection::Unselected
            }),
        )
    }

    fn paint(&self, scene: &mut UiScene) {
        draw_row_background(scene, self.layout.bounds(), self.style, self.state);

        if let Some(bounds) = self.layout.disclosure_bounds() {
            let icon = match self.layout.item().expansion() {
                TreeItemExpansion::Expanded => icons::CHEVRON_DOWN,
                TreeItemExpansion::Collapsed => icons::CHEVRON_RIGHT,
                TreeItemExpansion::Leaf => unreachable!("leaf has no disclosure geometry"),
            };
            scene.draw_component(&FilesTreeDisclosure::new(
                bounds,
                icon,
                self.style.text_muted,
            ));
        }

        let content = self.layout.content_bounds();
        let icon_bounds = Rect::from_xywh(
            content.origin.x,
            content.origin.y + (FILE_LIST_ROW_HEIGHT - ICON_SIZE) * 0.5,
            ICON_SIZE,
            ICON_SIZE,
        );
        if self.layout.item().expansion().is_branch() {
            scene.draw_component(&FilesTreeIcon::new(
                icon_bounds,
                icons::FILES,
                self.style.text_muted,
            ));
        }

        let text_x = icon_bounds.right() + 6.0;
        let text_bounds = Rect::from_xywh(
            text_x,
            content.origin.y + 4.0,
            (self.layout.bounds().right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        );
        scene.draw_component(&FilesTreeLabel::new(
            text_bounds,
            self.label,
            self.style.text,
        ));
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        draw_row_background(
            context.scene_mut(),
            self.layout.bounds(),
            self.style,
            self.state,
        );

        if let Some(bounds) = self.layout.disclosure_bounds() {
            let icon = match self.layout.item().expansion() {
                TreeItemExpansion::Expanded => icons::CHEVRON_DOWN,
                TreeItemExpansion::Collapsed => icons::CHEVRON_RIGHT,
                TreeItemExpansion::Leaf => unreachable!("leaf has no disclosure geometry"),
            };
            context.draw_component(&FilesTreeDisclosure::new(
                bounds,
                icon,
                self.style.text_muted,
            ));
        }

        let content = self.layout.content_bounds();
        let icon_bounds = Rect::from_xywh(
            content.origin.x,
            content.origin.y + (FILE_LIST_ROW_HEIGHT - ICON_SIZE) * 0.5,
            ICON_SIZE,
            ICON_SIZE,
        );
        if self.layout.item().expansion().is_branch() {
            context.draw_component(&FilesTreeIcon::new(
                icon_bounds,
                icons::FILES,
                self.style.text_muted,
            ));
        }

        let text_x = icon_bounds.right() + 6.0;
        let text_bounds = Rect::from_xywh(
            text_x,
            content.origin.y + 4.0,
            (self.layout.bounds().right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        );
        context.draw_component(&FilesTreeLabel::new(
            text_bounds,
            self.label,
            self.style.text,
        ));
    }
}

struct FilesTreeDisclosure {
    bounds: Rect,
    icon: Icon,
    color: Color,
}

impl FilesTreeDisclosure {
    const fn new(bounds: Rect, icon: Icon, color: Color) -> Self {
        Self {
            bounds,
            icon,
            color,
        }
    }
}

impl Component for FilesTreeDisclosure {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesTreeDisclosure").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_icon(PaintIcon::new(self.icon, self.bounds, self.color));
    }
}

struct FilesTreeIcon {
    bounds: Rect,
    icon: Icon,
    color: Color,
}

impl FilesTreeIcon {
    const fn new(bounds: Rect, icon: Icon, color: Color) -> Self {
        Self {
            bounds,
            icon,
            color,
        }
    }
}

impl Component for FilesTreeIcon {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesTreeIcon")
            .in_bounds(self.bounds)
            .with_inspection_label(self.icon.id().as_str())
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_icon(PaintIcon::new(self.icon, self.bounds, self.color));
    }
}

struct FilesTreeLabel<'a> {
    bounds: Rect,
    label: &'a str,
    color: Color,
}

impl<'a> FilesTreeLabel<'a> {
    const fn new(bounds: Rect, label: &'a str, color: Color) -> Self {
        Self {
            bounds,
            label,
            color,
        }
    }
}

impl Component for FilesTreeLabel<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesTreeLabel")
            .in_bounds(self.bounds)
            .with_inspection_label(self.label)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_text(TextBlock::new(
            self.label,
            self.bounds.origin,
            self.bounds.size,
            TextStyle::new(12.0, self.color).with_line_height(18.0),
        ));
    }
}

pub(super) fn file_row_state(selected: bool, hovered: bool) -> FileRowState {
    if selected {
        FileRowState::Selected
    } else if hovered {
        FileRowState::Hovered
    } else {
        FileRowState::Resting
    }
}

pub(super) fn draw_row_background(
    scene: &mut UiScene,
    bounds: Rect,
    style: &FilesPaneStyle,
    state: FileRowState,
) {
    let background = match state {
        FileRowState::Resting => return,
        FileRowState::Hovered => style.hovered_background,
        FileRowState::Selected => style.selected_background,
    };
    scene.draw_rect(PaintRect::new(bounds, background));
}
