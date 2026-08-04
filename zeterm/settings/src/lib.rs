//! Product-level Settings page composition.
//!
//! This crate owns the Settings workbench's retained layout and presentation contract. It does
//! not load or persist configuration and it does not know about a native window. Hosts provide
//! the search input, action availability, palette, and parent window identity, then execute the
//! element activations emitted by the shared interaction frame.

use zeta_icons::Icon;
use zeta_icons::icons;
use zeta_ui::ActionBar;
use zeta_ui::ActionBarButton;
use zeta_ui::ActionBarItem;
use zeta_ui::ActionBarStyle;
use zeta_ui::Border;
use zeta_ui::Button;
use zeta_ui::ButtonSelection;
use zeta_ui::ButtonState;
use zeta_ui::ButtonStyle;
use zeta_ui::CaretVisibility;
use zeta_ui::Color;
use zeta_ui::Component;
use zeta_ui::ComponentContext;
use zeta_ui::ComponentElement;
use zeta_ui::ComputedElement;
use zeta_ui::Edges;
use zeta_ui::Element;
use zeta_ui::InputBoxState;
use zeta_ui::InteractionRegion;
use zeta_ui::PaintRect;
use zeta_ui::Rect;
use zeta_ui::SearchBox;
use zeta_ui::SearchBoxStyle;
use zeta_ui::TextBlock;
use zeta_ui::TextInput;
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

const SETTINGS_SCOPE: u32 = 9;
const RAIL_WIDTH: f32 = 216.0;
const DEFAULT_HEADER_HEIGHT: f32 = 32.0;
const ACTION_BAR_HEIGHT: f32 = 56.0;
const PAGE_INSET: f32 = 28.0;
const NAV_INSET: f32 = 20.0;
const NAV_ITEM_HEIGHT: f32 = 34.0;
const NAV_ITEM_GAP: f32 = 4.0;
const NAV_TOP: f32 = 62.0;
const SEARCH_WIDTH: f32 = 320.0;
const CLOSE_SIZE: f32 = 32.0;

/// Root element for the Settings page and modal focus boundary.
pub const SETTINGS_PAGE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 1);
/// Search input in the Settings header.
pub const SETTINGS_SEARCH_INPUT: ElementId = ElementId::scoped(SETTINGS_SCOPE, 2);
/// Header close action.
pub const SETTINGS_CLOSE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 3);
/// Returns from the active Settings page to the host surface.
pub const SETTINGS_NAV_BACK: ElementId = ElementId::scoped(SETTINGS_SCOPE, 4);
/// Disabled placeholder navigation item for future general settings.
pub const SETTINGS_NAV_GENERAL: ElementId = ElementId::scoped(SETTINGS_SCOPE, 5);
/// Active Language Servers navigation item.
pub const SETTINGS_NAV_LANGUAGE_SERVERS: ElementId = ElementId::scoped(SETTINGS_SCOPE, 6);
/// Disabled placeholder navigation item for future appearance settings.
pub const SETTINGS_NAV_APPEARANCE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 7);
/// Disabled placeholder navigation item for future keybinding settings.
pub const SETTINGS_NAV_KEYBINDINGS: ElementId = ElementId::scoped(SETTINGS_SCOPE, 8);
/// Action-bar reset action for the active settings section.
pub const SETTINGS_RESET: ElementId = ElementId::scoped(SETTINGS_SCOPE, 9);
/// Action-bar save action for the active settings section.
pub const SETTINGS_SAVE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 10);

/// Capability state supplied by a Settings host for the two active page actions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsPageActionAvailability {
    reset_enabled: bool,
    save_enabled: bool,
}

impl SettingsPageActionAvailability {
    /// Starts with both page actions disabled.
    pub const fn none() -> Self {
        Self {
            reset_enabled: false,
            save_enabled: false,
        }
    }

    /// Sets whether the reset action is available.
    pub const fn with_reset_enabled(mut self, enabled: bool) -> Self {
        self.reset_enabled = enabled;
        self
    }

    /// Sets whether the save action is available.
    pub const fn with_save_enabled(mut self, enabled: bool) -> Self {
        self.save_enabled = enabled;
        self
    }
}

/// Resolved visual state for the two active page actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SettingsPageActionState {
    reset: ButtonState,
    save: ButtonState,
}

impl SettingsPageActionState {
    fn from_availability(
        availability: SettingsPageActionAvailability,
        dispatch: &UiDispatch,
    ) -> Self {
        Self {
            reset: button_state(SETTINGS_RESET, availability.reset_enabled, dispatch),
            save: button_state(SETTINGS_SAVE, availability.save_enabled, dispatch),
        }
    }

    pub const fn reset(self) -> ButtonState {
        self.reset
    }

    pub const fn save(self) -> ButtonState {
        self.save
    }
}

/// Palette and reusable component styles for the Settings page.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsPageStyle {
    background: Color,
    rail_background: Color,
    surface: Color,
    surface_raised: Color,
    border: Color,
    text: Color,
    accent: Color,
    search_box: SearchBoxStyle,
    nav_button: ButtonStyle,
    close_button: ButtonStyle,
    action_bar: ActionBarStyle,
}

impl SettingsPageStyle {
    /// Creates a Settings page style from host palette values and component contracts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        background: Color,
        rail_background: Color,
        surface: Color,
        surface_raised: Color,
        border: Color,
        text: Color,
        accent: Color,
        search_box: SearchBoxStyle,
        nav_button: ButtonStyle,
        close_button: ButtonStyle,
        action_bar: ActionBarStyle,
    ) -> Self {
        Self {
            background,
            rail_background,
            surface,
            surface_raised,
            border,
            text,
            accent,
            search_box,
            nav_button,
            close_button,
            action_bar,
        }
    }
}

/// Resolved top-level geometry owned by the Settings page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsPageLayout {
    viewport: Rect,
    rail: Rect,
    header: Rect,
    action_bar: Rect,
    content: Rect,
    search: Rect,
    close: Rect,
}

impl SettingsPageLayout {
    /// Resolves the Settings page for one logical viewport.
    pub fn for_viewport(viewport: Rect) -> Self {
        Self::for_viewport_with_header_height(viewport, DEFAULT_HEADER_HEIGHT)
    }

    /// Resolves the Settings page using the host's titlebar height for its header.
    pub fn for_viewport_with_header_height(viewport: Rect, header_height: f32) -> Self {
        let rail_width = RAIL_WIDTH.min((viewport.size.width * 0.32).max(168.0));
        let rail = Rect::from_xywh(
            viewport.origin.x,
            viewport.origin.y,
            rail_width,
            viewport.size.height,
        );
        let right_origin = viewport.origin.x + rail_width;
        let right_width = (viewport.size.width - rail_width).max(0.0);
        let header = Rect::from_xywh(
            right_origin,
            viewport.origin.y,
            right_width,
            header_height.max(0.0).min(viewport.size.height.max(0.0)),
        );
        let action_bar = Rect::from_xywh(
            right_origin,
            header.bottom(),
            right_width,
            ACTION_BAR_HEIGHT.min((viewport.bottom() - header.bottom()).max(0.0)),
        );
        let content = Rect::from_xywh(
            right_origin,
            action_bar.bottom(),
            right_width,
            (viewport.bottom() - action_bar.bottom()).max(0.0),
        );
        let search_width = SEARCH_WIDTH.min((header.size.width - PAGE_INSET * 2.0).max(1.0));
        let search = Rect::from_xywh(
            header.origin.x + PAGE_INSET,
            header.origin.y + (header.size.height - 36.0).max(0.0) * 0.5,
            search_width,
            36.0_f32.min(header.size.height.max(1.0)),
        );
        let close = Rect::from_xywh(
            header.right() - PAGE_INSET - CLOSE_SIZE,
            header.origin.y + (header.size.height - CLOSE_SIZE).max(0.0) * 0.5,
            CLOSE_SIZE.min(header.size.width.max(1.0)),
            CLOSE_SIZE.min(header.size.height.max(1.0)),
        );
        Self {
            viewport,
            rail,
            header,
            action_bar,
            content,
            search,
            close,
        }
    }

    pub const fn viewport(self) -> Rect {
        self.viewport
    }

    pub const fn rail(self) -> Rect {
        self.rail
    }

    pub const fn header(self) -> Rect {
        self.header
    }

    pub const fn action_bar(self) -> Rect {
        self.action_bar
    }

    /// Returns the content slot that a settings section should paint into.
    pub const fn content(self) -> Rect {
        self.content
    }

    pub const fn search(self) -> Rect {
        self.search
    }

    pub const fn close(self) -> Rect {
        self.close
    }

    fn navigation_bounds(self, index: usize) -> Rect {
        Rect::from_xywh(
            self.rail.origin.x + NAV_INSET,
            self.rail.origin.y + NAV_TOP + index as f32 * (NAV_ITEM_HEIGHT + NAV_ITEM_GAP),
            (self.rail.size.width - NAV_INSET * 2.0).max(1.0),
            NAV_ITEM_HEIGHT.min(self.rail.size.height.max(1.0)),
        )
    }
}

/// Retained Settings page shell with left navigation, header, action bar, and a content slot.
pub struct SettingsPage {
    layout: SettingsPageLayout,
    style: SettingsPageStyle,
    interaction_parent: Option<ElementId>,
    search_box: SearchBox,
    search_value: String,
    close_button: Button,
    navigation: [Button; 5],
    action_bar: ActionBar,
}

impl SettingsPage {
    /// Creates a visible Settings page using the default titlebar-sized header.
    pub fn new(
        viewport: Rect,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        style: SettingsPageStyle,
        actions: SettingsPageActionAvailability,
        dispatch: &UiDispatch,
        text_layout: &mut TextInputLayoutEngine,
    ) -> Self {
        Self::new_with_header_height(
            viewport,
            DEFAULT_HEADER_HEIGHT,
            search_input,
            caret_visibility,
            style,
            actions,
            dispatch,
            text_layout,
        )
    }

    /// Creates a Settings page whose header height is supplied by the hosting workbench.
    pub fn new_with_header_height(
        viewport: Rect,
        header_height: f32,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        style: SettingsPageStyle,
        actions: SettingsPageActionAvailability,
        dispatch: &UiDispatch,
        text_layout: &mut TextInputLayoutEngine,
    ) -> Self {
        let layout = SettingsPageLayout::for_viewport_with_header_height(viewport, header_height);
        let search_state = if dispatch.is_focused(SETTINGS_SEARCH_INPUT) {
            InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(SETTINGS_SEARCH_INPUT) {
            InputBoxState::Hovered
        } else {
            InputBoxState::Resting
        };
        let search_box = SearchBox::new(
            layout.search(),
            "Search settings",
            search_state,
            style.search_box.clone(),
            search_input,
            text_layout,
        );
        let close_button = Button::icon(
            layout.close(),
            icons::CLOSE,
            "Close settings",
            button_state(SETTINGS_CLOSE, true, dispatch),
            style.close_button.clone(),
        );
        let navigation = [
            navigation_button(
                layout.navigation_bounds(0),
                SETTINGS_NAV_BACK,
                icons::ARROW_LEFT,
                "Back",
                false,
                dispatch,
                &style.nav_button,
            ),
            navigation_button(
                layout.navigation_bounds(1),
                SETTINGS_NAV_GENERAL,
                icons::GEAR,
                "General",
                false,
                dispatch,
                &style.nav_button,
            ),
            navigation_button(
                layout.navigation_bounds(2),
                SETTINGS_NAV_LANGUAGE_SERVERS,
                icons::CODE,
                "Language Servers",
                true,
                dispatch,
                &style.nav_button,
            ),
            navigation_button(
                layout.navigation_bounds(3),
                SETTINGS_NAV_APPEARANCE,
                icons::APPEARANCE,
                "Appearance",
                false,
                dispatch,
                &style.nav_button,
            ),
            navigation_button(
                layout.navigation_bounds(4),
                SETTINGS_NAV_KEYBINDINGS,
                icons::COMMAND,
                "Keybindings",
                false,
                dispatch,
                &style.nav_button,
            ),
        ];
        let actions = SettingsPageActionState::from_availability(actions, dispatch);
        let action_bar = ActionBar::new(
            layout.action_bar(),
            zeta_ui::ActionBarOrientation::Horizontal,
            vec![
                ActionBarItem::Button(
                    ActionBarButton::label("Reset to Default", actions.reset())
                        .with_main_axis_extent(132.0),
                ),
                ActionBarItem::Button(
                    ActionBarButton::label("Save", actions.save()).with_main_axis_extent(76.0),
                ),
            ],
            style.action_bar.clone(),
        );
        Self {
            layout,
            style,
            interaction_parent: None,
            search_box,
            search_value: search_input.text().to_owned(),
            close_button,
            navigation,
            action_bar,
        }
    }

    pub const fn layout(&self) -> SettingsPageLayout {
        self.layout
    }

    /// Sets the host element that owns this page in the interaction tree.
    pub const fn with_parent(mut self, parent: ElementId) -> Self {
        self.interaction_parent = Some(parent);
        self
    }

    /// Returns the section content slot for the active settings page.
    pub const fn content_bounds(&self) -> Rect {
        self.layout.content()
    }

    pub const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let mut regions = Vec::new();
        let navigation = NavigationGroupId::new(SETTINGS_PAGE);
        regions.push(
            InteractionRegion::new(
                "SettingsSearchInput",
                SETTINGS_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search settings",
            )
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_navigation(navigation, NavigationAxis::Horizontal)
            .with_value(self.search_value.clone()),
        );
        regions.push(self.button_region(
            SETTINGS_CLOSE,
            self.layout.close(),
            "Close settings",
            navigation,
            NavigationAxis::Horizontal,
        ));
        regions.push(self.button_region(
            SETTINGS_NAV_BACK,
            self.navigation[0].bounds(),
            "Back",
            navigation,
            NavigationAxis::Vertical,
        ));
        regions.push(
            InteractionRegion::new(
                "SettingsNavigationItem",
                SETTINGS_NAV_LANGUAGE_SERVERS,
                self.navigation[2].bounds(),
                AccessibilityRole::Button,
                "Language Servers",
            )
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate)
            .with_navigation(navigation, NavigationAxis::Vertical)
            .with_selection(AccessibilitySelection::Selected),
        );
        if let Some(node) = self.action_node(
            SETTINGS_RESET,
            0,
            "Reset active settings section to defaults",
            navigation,
        ) {
            regions.push(node);
        }
        if let Some(node) =
            self.action_node(SETTINGS_SAVE, 1, "Save active settings section", navigation)
        {
            regions.push(node);
        }
        regions
    }

    fn button_region(
        &self,
        id: ElementId,
        bounds: Rect,
        label: &str,
        navigation: NavigationGroupId,
        axis: NavigationAxis,
    ) -> InteractionRegion {
        InteractionRegion::new(
            "SettingsButton",
            id,
            bounds,
            AccessibilityRole::Button,
            label,
        )
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate)
        .with_navigation(navigation, axis)
    }

    fn action_node(
        &self,
        id: ElementId,
        index: usize,
        label: &str,
        navigation: NavigationGroupId,
    ) -> Option<InteractionRegion> {
        self.action_bar
            .interactive_item_bounds(index)
            .map(|bounds| {
                self.button_region(id, bounds, label, navigation, NavigationAxis::Horizontal)
            })
    }
}

impl Component for SettingsPage {
    fn element(&self) -> ComponentElement {
        Element::leaf("SettingsPage")
            .in_bounds(self.layout.viewport())
            .with_identity(SETTINGS_PAGE)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        let node = UiNode::new(
            SETTINGS_PAGE,
            element.bounds(),
            AccessibilityRole::Group,
            "Settings",
        );
        Some(match self.interaction_parent {
            Some(parent) => node.with_parent(parent),
            None => node,
        })
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.set_modal_root(SETTINGS_PAGE);
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        self.paint(context.scene_mut());
    }

    fn paint(&self, scene: &mut UiScene) {
        let viewport = self.layout.viewport();
        scene.draw_rect(PaintRect::new(viewport, self.style.background));
        scene.draw_rect(
            PaintRect::new(self.layout.rail(), self.style.rail_background).with_border(
                Border::new(Edges::new(0.0, 1.0, 0.0, 0.0), self.style.border),
            ),
        );
        scene.draw_rect(
            PaintRect::new(self.layout.header(), self.style.surface).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.style.border,
            )),
        );
        scene.draw_rect(
            PaintRect::new(self.layout.action_bar(), self.style.surface_raised).with_border(
                Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.style.border),
            ),
        );
        draw_text(
            scene,
            "Settings",
            Rect::from_xywh(
                self.layout.rail().origin.x + PAGE_INSET,
                self.layout.rail().origin.y + 24.0,
                self.layout.rail().size.width - PAGE_INSET * 2.0,
                24.0,
            ),
            TextStyle::new(20.0, self.style.text)
                .with_line_height(24.0)
                .with_weight(zeta_ui::FontWeight::Bold),
        );
        for (index, button) in self.navigation.iter().enumerate() {
            scene.draw_component(button);
            if index == 2 {
                scene.draw_rect(PaintRect::new(
                    Rect::from_xywh(
                        button.bounds().origin.x,
                        button.bounds().origin.y,
                        3.0,
                        button.bounds().size.height,
                    ),
                    self.style.accent,
                ));
            }
        }
        scene.draw_component(&self.search_box);
        scene.draw_component(&self.close_button);
        scene.draw_component(&self.action_bar);
    }
}

fn navigation_button(
    bounds: Rect,
    id: ElementId,
    icon: Icon,
    label: &str,
    selected: bool,
    dispatch: &UiDispatch,
    style: &ButtonStyle,
) -> Button {
    let state = if !selected {
        ButtonState::Disabled
    } else {
        button_state(id, true, dispatch)
    };
    Button::icon_and_label(bounds, icon, label, state, style.clone()).with_selection(if selected {
        ButtonSelection::Selected
    } else {
        ButtonSelection::Unselected
    })
}

fn button_state(id: ElementId, enabled: bool, dispatch: &UiDispatch) -> ButtonState {
    if !enabled {
        ButtonState::Disabled
    } else if dispatch.is_pressed(id) {
        ButtonState::Pressed
    } else if dispatch.is_focused(id) {
        ButtonState::Focused
    } else if dispatch.is_hovered(id) {
        ButtonState::Hovered
    } else {
        ButtonState::Resting
    }
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
