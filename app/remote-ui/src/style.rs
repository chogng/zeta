//! Host-resolved colors and shared component styles for Remote overlays.

use zeta_icons::icons;
use zeta_ui::{
    CornerRadii, Edges, InputBoxStateColors, InputBoxStyle, ScrollViewStyle, SearchBoxStyle,
    TextStyle,
};

/// Colors and shared component styles needed by Remote overlays.
#[derive(Clone, Copy)]
pub struct RemoteUiStyle {
    pub surface: zeta_ui::Color,
    pub surface_raised: zeta_ui::Color,
    pub surface_hovered: zeta_ui::Color,
    pub border: zeta_ui::Color,
    pub text: zeta_ui::Color,
    pub text_muted: zeta_ui::Color,
    pub accent: zeta_ui::Color,
    pub error: zeta_ui::Color,
    pub terminal_selection: zeta_ui::Color,
    pub session_tab_highlight: zeta_ui::Color,
    file_list_scroll_view: ScrollViewStyle,
    picker_scroll_view: ScrollViewStyle,
}

impl RemoteUiStyle {
    /// Creates resolved style values for all Remote overlays.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        surface: zeta_ui::Color,
        surface_raised: zeta_ui::Color,
        surface_hovered: zeta_ui::Color,
        border: zeta_ui::Color,
        text: zeta_ui::Color,
        text_muted: zeta_ui::Color,
        accent: zeta_ui::Color,
        error: zeta_ui::Color,
        terminal_selection: zeta_ui::Color,
        session_tab_highlight: zeta_ui::Color,
        file_list_scroll_view: ScrollViewStyle,
        picker_scroll_view: ScrollViewStyle,
    ) -> Self {
        Self {
            surface,
            surface_raised,
            surface_hovered,
            border,
            text,
            text_muted,
            accent,
            error,
            terminal_selection,
            session_tab_highlight,
            file_list_scroll_view,
            picker_scroll_view,
        }
    }

    pub(crate) const fn file_list_scroll_view_style(self) -> ScrollViewStyle {
        self.file_list_scroll_view
    }

    pub(crate) const fn picker_scroll_view_style(self) -> ScrollViewStyle {
        self.picker_scroll_view
    }

    pub(crate) fn session_search_style(self) -> SearchBoxStyle {
        let input_box = InputBoxStyle::new(
            InputBoxStateColors::new(
                zeta_ui::Color::TRANSPARENT,
                zeta_ui::Color::TRANSPARENT,
                zeta_ui::Color::TRANSPARENT,
            ),
            InputBoxStateColors::new(
                zeta_ui::Color::TRANSPARENT,
                zeta_ui::Color::TRANSPARENT,
                zeta_ui::Color::TRANSPARENT,
            ),
            TextStyle::new(11.0, self.text).with_line_height(16.0),
            TextStyle::new(11.0, self.text_muted).with_line_height(16.0),
        )
        .with_border_width(0.0)
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_padding(Edges::new(4.0, 8.0, 4.0, 8.0))
        .with_selection_color(self.terminal_selection)
        .with_caret_color(self.accent)
        .with_preedit_underline_color(self.accent);
        SearchBoxStyle::new(input_box, icons::SEARCH, self.text_muted).with_icon_size(18.0)
    }
}
