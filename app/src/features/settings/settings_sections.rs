use zeta_keybinding::format_key_sequence;
use zeta_settings::SettingsPageSection;
use zui::ui::{
    Component, ComponentContext, ComponentElement, ComputedElement, Rect, UiDispatch, UiNode,
    UiScene,
};

use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::{KeyboardShortcutsState, keyboard_shortcut_rows};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_surface::WorkspaceSurfaceKind;

#[cfg(test)]
pub(crate) use zeta_settings::SETTINGS_SECTION_CONTENT;

pub(crate) struct SettingsSectionPane<'a> {
    bounds: Rect,
    section: SettingsPageSection,
    style: zeta_settings::SettingsSectionStyle,
    workspace_label: String,
    connection_label: String,
    surface_label: &'static str,
    keybinding_rows: Vec<zeta_settings::SettingsKeybindingRow>,
    keyboard_shortcuts_visible: bool,
    keybinding_diagnostics: &'a [String],
    theme_scheme: &'static str,
    theme_follows_system: bool,
    dispatch: &'a UiDispatch,
}

impl<'a> SettingsSectionPane<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        bounds: Rect,
        section: SettingsPageSection,
        palette: ShellPalette,
        workspace_context: &WorkspaceContext,
        workspace_surface: WorkspaceSurfaceKind,
        keybindings: &NativeKeybindings,
        keyboard_shortcuts: &KeyboardShortcutsState,
        keybinding_diagnostics: &'a [String],
        theme_scheme: zeta_theme::ColorScheme,
        theme_follows_system: bool,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let keybinding_rows = keyboard_shortcut_rows(keybindings)
            .into_iter()
            .map(|row| zeta_settings::SettingsKeybindingRow {
                element: row.element,
                label: row.label.to_owned(),
                value: row
                    .keybinding
                    .map(|binding| format_key_sequence(binding, keybindings.platform()))
                    .unwrap_or_else(|| "Unassigned".to_owned()),
            })
            .collect();
        Self {
            bounds,
            section,
            style: zeta_settings::SettingsSectionStyle {
                background: palette.background,
                surface: palette.surface,
                surface_raised: palette.surface_raised,
                surface_hovered: palette.surface_hovered,
                border: palette.border,
                text: palette.text,
                text_muted: palette.text_muted,
                accent: palette.accent,
                error: palette.error,
            },
            workspace_label: workspace_context.working_directory_label().to_owned(),
            connection_label: workspace_context.location_label().to_owned(),
            surface_label: match workspace_surface {
                WorkspaceSurfaceKind::Agent => "Agent workspace",
                WorkspaceSurfaceKind::Editor => "Editor",
                WorkspaceSurfaceKind::Terminal => "Terminal",
            },
            keybinding_rows,
            keyboard_shortcuts_visible: keyboard_shortcuts.is_visible(),
            keybinding_diagnostics,
            theme_scheme: match theme_scheme {
                zeta_theme::ColorScheme::Dark | zeta_theme::ColorScheme::HighContrastDark => "Dark",
                zeta_theme::ColorScheme::Light | zeta_theme::ColorScheme::HighContrastLight => {
                    "Light"
                }
            },
            theme_follows_system,
            dispatch,
        }
    }

    fn inner(&self) -> zeta_settings::SettingsSectionPane<'_> {
        zeta_settings::SettingsSectionPane::new(
            self.bounds,
            self.section,
            self.style,
            &self.workspace_label,
            &self.connection_label,
            self.surface_label,
            &self.keybinding_rows,
            self.keyboard_shortcuts_visible,
            self.keybinding_diagnostics,
            self.theme_scheme,
            self.theme_follows_system,
            self.dispatch,
        )
    }
}

impl Component for SettingsSectionPane<'_> {
    fn element(&self) -> ComponentElement {
        self.inner().element()
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        self.inner().interaction_node(element)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, element: &ComputedElement) {
        self.inner().compose(context, element);
    }

    fn paint(&self, scene: &mut UiScene) {
        Component::paint(&self.inner(), scene);
    }
}

#[cfg(test)]
#[path = "settings_sections_tests.rs"]
mod tests;
