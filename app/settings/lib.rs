//! Product-level Settings and Remote management presentation.
//!
//! This crate owns retained layout, state, and presentation for the Settings workbench and Remote
//! connection surfaces. It does not load or persist configuration, launch processes, or know about
//! a product window. Hosts provide feature snapshots, connection data, palette, and parent window
//! identity, then execute the actions emitted by the shared interaction frame.

mod keybindings;
mod keybindings_section;
mod navigation;
mod page;
mod pane;
mod remote;
mod section_layout;
mod sections;
mod state;

pub use keybindings::{
    KEYBOARD_SHORTCUTS, KEYBOARD_SHORTCUTS_CLOSE, KeyboardShortcutRow, KeyboardShortcutsState,
    ShortcutCommit, draw_keyboard_shortcuts_overlay, keyboard_shortcut_row_element,
    keyboard_shortcut_rows, settings_keybinding_rows,
};
pub use keybindings_section::{
    SETTINGS_KEYBINDINGS_LIST, SETTINGS_KEYBINDINGS_SCROLLBAR, SettingsKeybindingsViewport,
    SettingsScrollbarPointerOutcome,
};
pub use page::SettingsPage;
pub use pane::{
    AppearanceSettingsSnapshot, GeneralSettingsSnapshot, KeybindingSettingsSnapshot,
    RemoteSettingsSnapshot, SettingsFeatureSnapshot, SettingsPaneDrawResult, SettingsPaneStyle,
    SettingsPaneView, draw_settings_pane,
};
pub use remote::*;
pub use sections::{
    SETTINGS_SECTION_CONTENT, SettingsKeybindingRow, SettingsSectionPane, SettingsSectionStyle,
};
pub use state::{SettingsActivation, SettingsState};

use zeta_icons::icons;
use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonStyle;
use zeta_ui_components::InputBoxStateColors;
use zeta_ui_components::InputBoxStyle;
use zeta_ui_components::SearchBoxStyle;
use zeta_ui_theme::UiTheme;
use zui::ui::Border;
use zui::ui::Color;
use zui::ui::CornerRadii;
use zui::ui::Edges;
use zui::ui::ElementId;
use zui::ui::Rect;

const SETTINGS_SCOPE: u32 = 9;
const RAIL_WIDTH: f32 = 216.0;
const DEFAULT_HEADER_HEIGHT: f32 = 32.0;
const PAGE_INSET: f32 = 28.0;
const NAV_INSET: f32 = 20.0;
const NAV_ITEM_HEIGHT: f32 = 34.0;
const NAV_ITEM_GAP: f32 = 4.0;
const NAV_TOP: f32 = 62.0;
const SEARCH_WIDTH: f32 = 320.0;
const CLOSE_SIZE: f32 = 32.0;

/// Controls whether the Settings page owns a modal interaction boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsPageMode {
    /// The page is hosted inside a workbench surface and leaves sibling parts interactive.
    Surface,
    /// The page is hosted as a standalone modal surface and traps interaction in its subtree.
    #[default]
    Modal,
}

/// Root element for the Settings page and its optional modal focus boundary.
pub const SETTINGS_PAGE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 1);
/// Search input in the Settings header.
pub const SETTINGS_SEARCH_INPUT: ElementId = ElementId::scoped(SETTINGS_SCOPE, 2);
/// Header close action.
pub const SETTINGS_CLOSE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 3);
/// Returns from the active Settings page to the host surface.
pub const SETTINGS_NAV_BACK: ElementId = ElementId::scoped(SETTINGS_SCOPE, 4);
/// General application and workspace preferences.
pub const SETTINGS_NAV_GENERAL: ElementId = ElementId::scoped(SETTINGS_SCOPE, 5);
/// Remote workspace, connection, and Tunnel overview.
pub const SETTINGS_NAV_REMOTE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 6);
/// Appearance and theme preferences.
pub const SETTINGS_NAV_APPEARANCE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 7);
/// Keyboard shortcut preferences.
pub const SETTINGS_NAV_KEYBINDINGS: ElementId = ElementId::scoped(SETTINGS_SCOPE, 8);

/// The section currently projected into the Settings content slot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsPageSection {
    /// Application and workspace defaults.
    #[default]
    General,
    /// Theme and visual preferences.
    Appearance,
    /// Keyboard shortcut preferences.
    Keybindings,
    /// Remote workspace, connection, and Tunnel overview.
    Remote,
}

impl SettingsPageSection {
    const fn navigation_index(self) -> usize {
        match self {
            Self::General => 1,
            Self::Appearance => 2,
            Self::Keybindings => 3,
            Self::Remote => 4,
        }
    }
}

/// Palette and reusable component styles for the Settings page.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsPageStyle {
    background: Color,
    rail_background: Color,
    surface: Color,
    border: Color,
    text: Color,
    accent: Color,
    search_box: SearchBoxStyle,
    nav_button: ButtonStyle,
    close_button: ButtonStyle,
}

impl SettingsPageStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        let search_input = InputBoxStyle::new(
            InputBoxStateColors::new(
                theme.side_bar_background,
                theme.list_hover_background,
                theme.content_background,
            ),
            InputBoxStateColors::new(theme.border, theme.border, theme.accent),
            zui::ui::TextStyle::new(theme.font_size_body(), theme.foreground)
                .with_line_height(18.0),
            zui::ui::TextStyle::new(theme.font_size_body(), theme.muted_foreground)
                .with_line_height(18.0),
        )
        .with_border_width(1.0)
        .with_corner_radii(CornerRadii::uniform(6.0))
        .with_padding(Edges::new(8.0, 10.0, 8.0, 10.0))
        .with_selection_color(theme.text_selection_background)
        .with_caret_color(theme.accent)
        .with_preedit_underline_color(theme.accent);
        let nav_button = ButtonStyle::new(
            ButtonBackgrounds::new(Color::TRANSPARENT)
                .with_hovered(theme.list_hover_background)
                .with_focused(theme.list_hover_background)
                .with_pressed(theme.border),
            zui::ui::TextStyle::new(theme.font_size_body(), theme.foreground)
                .with_line_height(18.0),
        )
        .with_selected_backgrounds(ButtonBackgrounds::new(theme.list_active_background))
        .with_disabled_text_style(
            zui::ui::TextStyle::new(theme.font_size_body(), theme.muted_foreground)
                .with_line_height(18.0),
        )
        .with_corner_radii(CornerRadii::uniform(5.0))
        .with_padding(Edges::new(7.0, 10.0, 7.0, 10.0));
        let close_button = ButtonStyle::new(
            ButtonBackgrounds::new(Color::TRANSPARENT)
                .with_hovered(theme.list_hover_background)
                .with_focused(theme.list_hover_background)
                .with_pressed(theme.border),
            zui::ui::TextStyle::new(theme.font_size_body(), theme.foreground)
                .with_line_height(18.0),
        )
        .with_border(Border::uniform(0.0, Color::TRANSPARENT))
        .with_corner_radii(CornerRadii::uniform(5.0))
        .with_padding(Edges::uniform(6.0));
        Self::new(
            theme.workbench_background,
            theme.side_bar_background,
            theme.content_background,
            theme.border,
            theme.foreground,
            theme.accent,
            SearchBoxStyle::new(search_input, icons::SEARCH, theme.muted_foreground)
                .with_icon_size(16.0),
            nav_button,
            close_button,
        )
    }

    /// Creates a Settings page style from host palette values and component contracts.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        background: Color,
        rail_background: Color,
        surface: Color,
        border: Color,
        text: Color,
        accent: Color,
        search_box: SearchBoxStyle,
        nav_button: ButtonStyle,
        close_button: ButtonStyle,
    ) -> Self {
        Self {
            background,
            rail_background,
            surface,
            border,
            text,
            accent,
            search_box,
            nav_button,
            close_button,
        }
    }
}

/// Resolved top-level geometry owned by the Settings page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsPageLayout {
    viewport: Rect,
    rail: Rect,
    header: Rect,
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
        let content = Rect::from_xywh(
            right_origin,
            header.bottom(),
            right_width,
            (viewport.bottom() - header.bottom()).max(0.0),
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

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
