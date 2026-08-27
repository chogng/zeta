use super::SETTINGS_SECTION_CONTENT;
use super::SettingsSectionPane;
use crate::keybindings::NativeKeybindings;
use crate::keyboard_shortcuts::KeyboardShortcutsState;
use crate::keyboard_shortcuts::keyboard_shortcut_rows;
use crate::shell_style::SHELL_PALETTE;
use crate::workspace_context::WorkspaceContext;
use crate::workspace_surface::WorkspaceSurfaceKind;
use zeta_settings::SettingsPageSection;
use zui::ui::InteractionFrame;
use zui::ui::Rect;
use zui::ui::UiDispatch;
use zui::ui::UiFrame;

#[test]
fn keybindings_section_projects_real_rows_and_shortcut_values() {
    let workspace = WorkspaceContext::fixture("~/Desktop/zeta", Some("main"), Some(2));
    let keybindings = NativeKeybindings::default();
    let keyboard_shortcuts = KeyboardShortcutsState::default();
    let dispatch = UiDispatch::default();
    let pane = SettingsSectionPane::new(
        Rect::from_xywh(216.0, 88.0, 784.0, 612.0),
        SettingsPageSection::Keybindings,
        SHELL_PALETTE,
        &workspace,
        WorkspaceSurfaceKind::Agent,
        &keybindings,
        &keyboard_shortcuts,
        &[],
        zeta_theme::ColorScheme::Light,
        true,
        &dispatch,
    );
    let mut frame = UiFrame::<InteractionFrame>::new(SHELL_PALETTE.background);
    frame.draw_component(&pane);

    let root = frame
        .interaction()
        .node(SETTINGS_SECTION_CONTENT)
        .expect("settings content root");
    assert_eq!(root.parent(), Some(zeta_settings::SETTINGS_PAGE));
    let first_row = keyboard_shortcut_rows(&keybindings)
        .into_iter()
        .next()
        .expect("bindable command row");
    assert!(frame.interaction().node(first_row.element).is_some());
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Copy")
    );
    assert!(
        frame
            .scene()
            .text_blocks()
            .iter()
            .any(|text| text.text() == "Keybindings")
    );
}
