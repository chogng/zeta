//! Retained Settings page component implementation.

use super::DEFAULT_HEADER_HEIGHT;
use super::SETTINGS_CLOSE;
use super::SETTINGS_NAV_APPEARANCE;
use super::SETTINGS_NAV_BACK;
use super::SETTINGS_NAV_GENERAL;
use super::SETTINGS_NAV_KEYBINDINGS;
use super::SETTINGS_NAV_LANGUAGE_SERVERS;
use super::SETTINGS_PAGE;
use super::SETTINGS_RESET;
use super::SETTINGS_SAVE;
use super::SETTINGS_SEARCH_INPUT;
use super::SettingsPageActionAvailability;
use super::SettingsPageLayout;
use super::SettingsPageMode;
use super::SettingsPageSection;
use super::SettingsPageStyle;
use super::navigation::SettingsPageActionState;
use super::navigation::button_state;
use super::navigation::navigation_buttons;

use zeta_icons::icons;
use zeta_ui::ActionBar;
use zeta_ui::ActionBarButton;
use zeta_ui::ActionBarItem;
use zeta_ui::Button;
use zeta_ui::CaretVisibility;
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
use zeta_ui::TextBlock;
use zeta_ui::TextInput;
use zeta_ui::TextInputLayoutEngine;
use zeta_ui::TextStyle;
use zeta_ui::UiScene;
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::CursorFeedback;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::NavigationAxis;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::UiDispatch;
use zui::ui::UiNode;

/// Retained Settings page shell with left navigation, header, action bar, and a content slot.
pub struct SettingsPage {
    layout: SettingsPageLayout,
    style: SettingsPageStyle,
    mode: SettingsPageMode,
    section: SettingsPageSection,
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
        Self::new_with_header_height_and_section(
            viewport,
            header_height,
            search_input,
            caret_visibility,
            style,
            actions,
            SettingsPageSection::default(),
            dispatch,
            text_layout,
        )
    }

    /// Creates a Settings page with an explicitly selected navigation section.
    pub fn new_with_header_height_and_section(
        viewport: Rect,
        header_height: f32,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        style: SettingsPageStyle,
        actions: SettingsPageActionAvailability,
        section: SettingsPageSection,
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
        let navigation = navigation_buttons(layout, section, dispatch, &style.nav_button);
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
            mode: SettingsPageMode::default(),
            section,
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

    /// Selects whether the page is embedded in a workbench surface or acts as a modal page.
    pub const fn with_mode(mut self, mode: SettingsPageMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns the section selected in the navigation rail.
    pub const fn section(&self) -> SettingsPageSection {
        self.section
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
        for (index, (id, label)) in [
            (SETTINGS_NAV_GENERAL, "General"),
            (SETTINGS_NAV_LANGUAGE_SERVERS, "Language Servers"),
            (SETTINGS_NAV_APPEARANCE, "Appearance"),
            (SETTINGS_NAV_KEYBINDINGS, "Keybindings"),
        ]
        .into_iter()
        .enumerate()
        {
            let selected = index + 1 == self.section.navigation_index();
            regions.push(
                InteractionRegion::new(
                    "SettingsNavigationItem",
                    id,
                    self.navigation[index + 1].bounds(),
                    AccessibilityRole::Button,
                    label,
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Vertical)
                .with_selection(if selected {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
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
        if self.mode == SettingsPageMode::Modal {
            context.set_modal_root(SETTINGS_PAGE);
        }
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
                zeta_ui::Border::new(Edges::new(0.0, 1.0, 0.0, 0.0), self.style.border),
            ),
        );
        scene.draw_rect(
            PaintRect::new(self.layout.header(), self.style.surface).with_border(
                zeta_ui::Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.style.border),
            ),
        );
        scene.draw_rect(
            PaintRect::new(self.layout.action_bar(), self.style.surface_raised).with_border(
                zeta_ui::Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.style.border),
            ),
        );
        draw_text(
            scene,
            "Settings",
            Rect::from_xywh(
                self.layout.rail().origin.x + super::PAGE_INSET,
                self.layout.rail().origin.y + 24.0,
                self.layout.rail().size.width - super::PAGE_INSET * 2.0,
                24.0,
            ),
            TextStyle::new(20.0, self.style.text)
                .with_line_height(24.0)
                .with_weight(zeta_ui::FontWeight::Bold),
        );
        for (index, button) in self.navigation.iter().enumerate() {
            scene.draw_component(button);
            if index == self.section.navigation_index() {
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

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}
