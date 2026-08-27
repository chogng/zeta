use zui::ui::CaretVisibility;
use zui::ui::ComponentContext;
use zui::ui::ElementId;
use zui::ui::Rect;
use zui::ui::TextInputLayoutEngine;
use zui::ui::UiDispatch;

use crate::SETTINGS_SEARCH_INPUT;
use crate::SettingsKeybindingRow;
use crate::SettingsPage;
use crate::SettingsPageMode;
use crate::SettingsPageStyle;
use crate::SettingsSectionPane;
use crate::SettingsSectionStyle;
use crate::SettingsState;

/// Workspace facts rendered by the General section.
pub struct GeneralSettingsSnapshot<'a> {
    pub workspace_label: &'a str,
    pub connection_label: &'a str,
    pub surface_label: &'a str,
}

/// Theme facts rendered by the Appearance section.
pub struct AppearanceSettingsSnapshot<'a> {
    pub scheme: &'a str,
    pub follows_system: bool,
}

/// Command rows and diagnostics rendered by the Keybindings section.
pub struct KeybindingSettingsSnapshot<'a> {
    pub keybinding_rows: &'a [SettingsKeybindingRow],
    pub keybinding_diagnostics: &'a [String],
}

/// Section-scoped read-only values supplied by feature owners for one Settings frame.
pub struct SettingsFeatureSnapshot<'a> {
    pub general: GeneralSettingsSnapshot<'a>,
    pub appearance: AppearanceSettingsSnapshot<'a>,
    pub keybindings: KeybindingSettingsSnapshot<'a>,
}

/// Styles for the page shell and its concrete sections.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsPaneStyle {
    page: SettingsPageStyle,
    section: SettingsSectionStyle,
}

impl SettingsPaneStyle {
    pub const fn new(page: SettingsPageStyle, section: SettingsSectionStyle) -> Self {
        Self { page, section }
    }
}

/// Inputs needed to render one Settings frame without transferring feature ownership.
pub struct SettingsPaneView<'a> {
    pub state: &'a SettingsState,
    pub features: SettingsFeatureSnapshot<'a>,
    pub caret_visibility: CaretVisibility,
    pub dispatch: &'a UiDispatch,
}

/// Geometry returned to the product host for input-method integration.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SettingsPaneDrawResult {
    pub ime_cursor_area: Option<Rect>,
}

pub fn draw_settings_pane(
    context: &mut ComponentContext<'_, '_>,
    bounds: Rect,
    header_height: f32,
    parent: ElementId,
    view: SettingsPaneView<'_>,
    style: SettingsPaneStyle,
    text_layout: &mut TextInputLayoutEngine,
) -> SettingsPaneDrawResult {
    let section = view.state.section();
    let page = SettingsPage::new_with_header_height_and_section(
        bounds,
        header_height,
        view.state.search_input(),
        view.caret_visibility,
        style.page,
        section,
        view.dispatch,
        text_layout,
    )
    .with_parent(parent)
    .with_mode(SettingsPageMode::Surface);
    let result = SettingsPaneDrawResult {
        ime_cursor_area: view
            .dispatch
            .is_focused(SETTINGS_SEARCH_INPUT)
            .then(|| page.search_caret_bounds())
            .flatten(),
    };
    let content = page.content_bounds();
    context.draw_component(&page);
    context.draw_component(&SettingsSectionPane::new(
        content,
        section,
        style.section,
        view.features.general.workspace_label,
        view.features.general.connection_label,
        view.features.general.surface_label,
        view.features.keybindings.keybinding_rows,
        view.state.keyboard_shortcuts().is_visible(),
        view.features.keybindings.keybinding_diagnostics,
        view.features.appearance.scheme,
        view.features.appearance.follows_system,
        view.dispatch,
    ));
    result
}
