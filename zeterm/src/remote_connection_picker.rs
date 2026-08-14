use zeta_remote_connections::RemoteConnectionEntry;
use zeta_remote_connections::RemoteConnectionName;
use zeta_ui::ButtonBackgrounds;
use zeta_ui::ButtonState;
use zeta_ui::ButtonStyle;
use zeta_ui::CaretVisibility;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::ContextViewAnchorPosition;
use zeta_ui::ContextViewPlacement;
use zeta_ui::CornerRadii;
use zeta_ui::Dropdown;
use zeta_ui::DropdownItem;
use zeta_ui::DropdownScrollConfiguration;
use zeta_ui::DropdownSelection;
use zeta_ui::DropdownStyle;
use zeta_ui::Edges;
use zeta_ui::Element;
use zeta_ui::InputBoxState;
use zeta_ui::InteractionRegion;
use zeta_ui::Rect;
use zeta_ui::ScrollAxis;
use zeta_ui::ScrollCommand;
use zeta_ui::ScrollMetrics;
use zeta_ui::ScrollState;
use zeta_ui::SearchBox;
use zeta_ui::Size;
use zeta_ui::TextInput;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputCompositionEvent;
use zeta_ui::TextInputLayoutEngine;
use zeta_ui::TextStyle;
use zeta_ui::UiScene;
use zui::AccessibilityRole;
use zui::AccessibilitySelection;
use zui::CursorFeedback;
use zui::ElementId;
use zui::FocusBehavior;
use zui::NavigationAxis;
use zui::NavigationGroupId;
use zui::NodeAction;
use zui::UiDispatch;
use zui::UiNode;

use crate::shell_interaction::WINDOW;
use crate::shell_style::ShellPalette;

const REMOTE_CONNECTION_PICKER_SCOPE: u32 = 9;
const REMOTE_CONNECTION_PICKER: ElementId = ElementId::scoped(REMOTE_CONNECTION_PICKER_SCOPE, 1);
pub(crate) const REMOTE_CONNECTION_SEARCH_INPUT: ElementId =
    ElementId::scoped(REMOTE_CONNECTION_PICKER_SCOPE, 2);
const FIRST_REMOTE_CONNECTION_ITEM: u32 = 3;
const PICKER_VISIBLE_ITEM_COUNT: usize = 8;
const PICKER_CONTENT_WIDTH: f32 = 440.0;
pub(crate) const REMOTE_CONNECTION_ITEM_HEIGHT: f32 = 30.0;
const PICKER_SEARCH_ROW_HEIGHT: f32 = 36.0;
const PICKER_SEARCH_INSET: f32 = 4.0;
const PICKER_VIEWPORT_MARGIN: f32 = 6.0;
const PICKER_ANCHOR_GAP: f32 = 4.0;

#[derive(Clone, Debug, Eq, PartialEq)]
struct RemoteConnectionPickerItem {
    label: String,
    action: Option<RemoteConnectionPickerAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RemoteConnectionPickerAction {
    Manage,
    ManageTunnels,
    Connect(RemoteConnectionName),
}

#[derive(Clone, Debug, PartialEq)]
struct OpenRemoteConnectionPicker {
    anchor: Rect,
    connections: Vec<RemoteConnectionEntry>,
    tunnels_available: bool,
    restore_focus: Option<ElementId>,
}

/// Product-owned picker state over the shared credential-free Remote target catalog.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RemoteConnectionPickerState {
    open: Option<OpenRemoteConnectionPicker>,
    search_input: TextInput,
    scroll: ScrollState,
}

impl RemoteConnectionPickerState {
    pub(crate) fn open(
        &mut self,
        anchor: Rect,
        mut connections: Vec<RemoteConnectionEntry>,
        tunnels_available: bool,
        restore_focus: Option<ElementId>,
    ) {
        connections.sort_by(|left, right| left.name().cmp(right.name()));
        self.search_input.take_text();
        self.open = Some(OpenRemoteConnectionPicker {
            anchor,
            connections,
            tunnels_available,
            restore_focus,
        });
        self.scroll = ScrollState::default();
    }

    pub(crate) const fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub(crate) fn dismiss(&mut self) -> Option<ElementId> {
        self.open.take().and_then(|open| open.restore_focus)
    }

    pub(crate) const fn search_input(&self) -> &TextInput {
        &self.search_input
    }

    pub(crate) fn apply_search(&mut self, command: TextInputCommand) {
        self.search_input.apply(command);
        self.scroll = ScrollState::default();
    }

    pub(crate) fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search_input.apply_composition(event);
        self.scroll = ScrollState::default();
    }

    pub(crate) fn cancel_search_composition(&mut self) {
        self.search_input.cancel_composition();
    }

    pub(crate) fn selected_search_text(&self) -> Option<&str> {
        self.search_input.selected_text()
    }

    pub(crate) const fn scroll_state(&self) -> ScrollState {
        self.scroll
    }

    pub(crate) fn apply_scroll(&mut self, command: ScrollCommand, metrics: ScrollMetrics) -> bool {
        self.scroll.apply(command, metrics, ScrollAxis::Vertical)
    }

    pub(crate) fn ensure_item_visible(&mut self, index: usize, metrics: ScrollMetrics) -> bool {
        self.apply_scroll(
            ScrollCommand::EnsureVisible(Rect::from_xywh(
                0.0,
                index as f32 * REMOTE_CONNECTION_ITEM_HEIGHT,
                metrics.content().width,
                REMOTE_CONNECTION_ITEM_HEIGHT,
            )),
            metrics,
        )
    }

    pub(crate) fn first_action_id(&self) -> Option<ElementId> {
        self.items().iter().enumerate().find_map(|(index, item)| {
            item.action
                .as_ref()
                .map(|_| remote_connection_item_id(index))
        })
    }

    pub(crate) fn is_picker_element(&self, id: ElementId) -> bool {
        id == REMOTE_CONNECTION_PICKER
            || id == REMOTE_CONNECTION_SEARCH_INPUT
            || self
                .items()
                .iter()
                .enumerate()
                .any(|(index, _)| remote_connection_item_id(index) == id)
    }

    pub(crate) fn item_index(&self, id: ElementId) -> Option<usize> {
        self.items()
            .iter()
            .enumerate()
            .find_map(|(index, _)| (remote_connection_item_id(index) == id).then_some(index))
    }

    pub(crate) fn activate(&self, index: usize) -> Option<RemoteConnectionPickerAction> {
        self.items().get(index)?.action.clone()
    }

    fn items(&self) -> Vec<RemoteConnectionPickerItem> {
        let Some(open) = self.open.as_ref() else {
            return Vec::new();
        };
        let query = self.search_input.text().trim().to_ascii_lowercase();
        let mut items = Vec::new();
        if query.is_empty() || "manage remote connections".contains(&query) {
            items.push(RemoteConnectionPickerItem {
                label: "Manage Remote connections…".into(),
                action: Some(RemoteConnectionPickerAction::Manage),
            });
        }
        if open.tunnels_available && (query.is_empty() || "manage remote tunnels".contains(&query))
        {
            items.push(RemoteConnectionPickerItem {
                label: "Manage Remote tunnels…".into(),
                action: Some(RemoteConnectionPickerAction::ManageTunnels),
            });
        }
        items.extend(
            open.connections
                .iter()
                .filter(|entry| {
                    query.is_empty()
                        || entry.name().as_str().contains(&query)
                        || entry
                            .target()
                            .host()
                            .as_str()
                            .to_ascii_lowercase()
                            .contains(&query)
                        || entry
                            .target()
                            .workspace()
                            .as_str()
                            .to_ascii_lowercase()
                            .contains(&query)
                })
                .map(|entry| RemoteConnectionPickerItem {
                    label: format!(
                        "{} · {} · {}",
                        entry.name().as_str(),
                        entry.target().host().as_str(),
                        entry.target().workspace().as_str()
                    ),
                    action: Some(RemoteConnectionPickerAction::Connect(entry.name().clone())),
                }),
        );
        if !items.is_empty() {
            return items;
        }
        vec![RemoteConnectionPickerItem {
            label: if open.connections.is_empty() {
                "No matching Remote actions".into()
            } else {
                "No matching Remote connections".into()
            },
            action: None,
        }]
    }
}

pub(crate) struct RemoteConnectionPicker {
    dropdown: Dropdown,
    search_box: SearchBox,
    search_value: String,
    items: Vec<RemoteConnectionPickerItem>,
}

impl RemoteConnectionPicker {
    pub(crate) fn new(
        viewport: Rect,
        state: &RemoteConnectionPickerState,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Option<Self> {
        let open = state.open.as_ref()?;
        let items = state.items();
        let resting_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT);
        let selected_backgrounds = ButtonBackgrounds::new(palette.session_tab_highlight)
            .with_hovered(palette.session_tab_highlight)
            .with_focused(palette.session_tab_highlight)
            .with_pressed(palette.border);
        let button_style = ButtonStyle::new(
            resting_backgrounds,
            TextStyle::new(13.0, palette.text).with_line_height(18.0),
        )
        .with_selected_backgrounds(selected_backgrounds)
        .with_corner_radii(CornerRadii::uniform(2.0))
        .with_padding(Edges::new(0.0, 10.0, 0.0, 10.0));
        let dropdown_items = items
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let id = remote_connection_item_id(index);
                let state = if entry.action.is_none() {
                    ButtonState::Disabled
                } else if dispatch.is_pressed(id) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(id) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(id) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                DropdownItem::new(entry.label.clone(), state)
            })
            .collect();
        let selection = items
            .iter()
            .enumerate()
            .find_map(|(index, _)| {
                let id = remote_connection_item_id(index);
                (dispatch.is_pressed(id) || dispatch.is_hovered(id) || dispatch.is_focused(id))
                    .then_some(index)
            })
            .map(DropdownSelection::Item)
            .unwrap_or(DropdownSelection::None);
        let dropdown = Dropdown::new_scrollable(
            viewport,
            open.anchor,
            dropdown_items,
            DropdownStyle::new(
                palette.surface,
                button_style,
                Size::new(PICKER_CONTENT_WIDTH, REMOTE_CONNECTION_ITEM_HEIGHT),
            )
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_header_height(PICKER_SEARCH_ROW_HEIGHT)
            .with_placement(
                ContextViewPlacement::new()
                    .with_position(ContextViewAnchorPosition::Before)
                    .with_gap(PICKER_ANCHOR_GAP)
                    .with_viewport_margin(PICKER_VIEWPORT_MARGIN),
            ),
            DropdownScrollConfiguration::new(
                state.scroll_state(),
                PICKER_VISIBLE_ITEM_COUNT,
                palette.picker_scroll_view_style(),
            ),
        )
        .with_selection(selection);
        let header_bounds = dropdown
            .header_bounds()
            .expect("Remote connection picker reserves a search row");
        let search_bounds = Rect::from_xywh(
            header_bounds.origin.x + PICKER_SEARCH_INSET,
            header_bounds.origin.y + PICKER_SEARCH_INSET,
            (header_bounds.size.width - PICKER_SEARCH_INSET * 2.0).max(1.0),
            (header_bounds.size.height - PICKER_SEARCH_INSET * 2.0).max(1.0),
        );
        let search_state = if dispatch.is_focused(REMOTE_CONNECTION_SEARCH_INPUT) {
            InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(REMOTE_CONNECTION_SEARCH_INPUT) {
            InputBoxState::Hovered
        } else {
            InputBoxState::Resting
        };
        let search_box = SearchBox::new(
            search_bounds,
            "Search Remote connections...",
            search_state,
            palette.session_search_style(),
            state.search_input(),
            text_layout,
        );
        Some(Self {
            dropdown,
            search_box,
            search_value: state.search_input().text().into(),
            items,
        })
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation_group = NavigationGroupId::new(REMOTE_CONNECTION_PICKER);
        let mut regions = vec![
            InteractionRegion::new(
                "RemoteConnectionSearchInput",
                REMOTE_CONNECTION_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search Remote connections",
            )
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation_group, NavigationAxis::Vertical)
            .with_value(&self.search_value),
        ];
        for (index, item) in self.items.iter().enumerate() {
            if item.action.is_none() {
                continue;
            }
            let Some(bounds) = self.dropdown.interactive_item_bounds(index) else {
                continue;
            };
            regions.push(
                InteractionRegion::new(
                    "RemoteConnectionItem",
                    remote_connection_item_id(index),
                    bounds,
                    AccessibilityRole::MenuItem,
                    item.label.clone(),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation_group, NavigationAxis::Vertical)
                .with_selection(if self.dropdown.selected_index() == Some(index) {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        regions
    }

    #[cfg(test)]
    pub(crate) const fn bounds(&self) -> Rect {
        self.dropdown.bounds()
    }

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    pub(crate) const fn item_viewport_bounds(&self) -> Rect {
        self.dropdown.item_viewport_bounds()
    }

    pub(crate) fn scroll_metrics(&self) -> Option<ScrollMetrics> {
        self.dropdown.scroll_metrics()
    }
}

impl Component for RemoteConnectionPicker {
    fn element(&self) -> ComponentElement {
        Element::leaf("RemoteConnectionPicker")
            .in_bounds(self.dropdown.bounds())
            .with_identity(REMOTE_CONNECTION_PICKER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                REMOTE_CONNECTION_PICKER,
                element.bounds(),
                AccessibilityRole::Menu,
                "Open Remote connection",
            )
            .with_parent(WINDOW),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(REMOTE_CONNECTION_PICKER);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        self.dropdown
            .draw_components_with_header(context, |context, _bounds| {
                context.draw_component(&self.search_box);
            });
    }

    fn paint(&self, scene: &mut UiScene) {
        self.dropdown.paint_with_header(scene, |scene, _bounds| {
            scene.draw_component(&self.search_box)
        });
    }
}

pub(crate) fn remote_connection_item_id(index: usize) -> ElementId {
    ElementId::scoped(
        REMOTE_CONNECTION_PICKER_SCOPE,
        FIRST_REMOTE_CONNECTION_ITEM.saturating_add(index as u32),
    )
}

#[cfg(test)]
#[path = "remote_connection_picker_tests.rs"]
mod tests;
