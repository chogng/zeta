use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonStyle;
use zui::ui::Color;
use zui::ui::Edges;
use zui::ui::TextStyle;

/// Theme values required by Workspace Pane navigation.
#[derive(Clone)]
pub struct WorkspaceNavigationStyle {
    pub surface_raised: Color,
    pub text: Color,
    pub surface_hovered: Color,
    pub session_tab_highlight: Color,
}

impl WorkspaceNavigationStyle {
    pub fn new(
        surface_raised: Color,
        text: Color,
        surface_hovered: Color,
        session_tab_highlight: Color,
    ) -> Self {
        Self {
            surface_raised,
            text,
            surface_hovered,
            session_tab_highlight,
        }
    }

    pub fn button_style(&self) -> ButtonStyle {
        let backgrounds = ButtonBackgrounds::new(self.surface_raised)
            .with_hovered(self.surface_hovered)
            .with_focused(self.surface_hovered)
            .with_pressed(self.session_tab_highlight);
        let selected = ButtonBackgrounds::new(self.session_tab_highlight)
            .with_hovered(self.session_tab_highlight)
            .with_focused(self.session_tab_highlight)
            .with_pressed(self.session_tab_highlight);
        ButtonStyle::new(backgrounds, TextStyle::new(11.0, self.text))
            .with_selected_backgrounds(selected)
            .with_padding(Edges::new(0.0, 6.0, 0.0, 6.0))
    }
}
