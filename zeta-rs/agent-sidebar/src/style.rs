use zeta_ui::ButtonBackgrounds;
use zeta_ui::ButtonStyle;
use zeta_ui::Color;
use zeta_ui::CornerRadii;
use zeta_ui::Edges;
use zeta_ui::SearchBoxStyle;
use zeta_ui::TextStyle;

/// Theme projection shared by the Agent Sidebar's toolbar and pane navigation.
#[derive(Clone)]
pub struct AgentSidebarStyle {
    pub surface: Color,
    pub surface_raised: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub surface_hovered: Color,
    pub session_tab_highlight: Color,
    search: SearchBoxStyle,
}

impl AgentSidebarStyle {
    pub fn new(
        surface: Color,
        surface_raised: Color,
        border: Color,
        text: Color,
        text_muted: Color,
        surface_hovered: Color,
        session_tab_highlight: Color,
        search: SearchBoxStyle,
    ) -> Self {
        Self {
            surface,
            surface_raised,
            border,
            text,
            text_muted,
            surface_hovered,
            session_tab_highlight,
            search,
        }
    }

    pub fn search_style(&self) -> SearchBoxStyle {
        self.search.clone()
    }

    pub fn toolbar_button_style(&self) -> ButtonStyle {
        let backgrounds = ButtonBackgrounds::new(Color::TRANSPARENT)
            .with_hovered(self.surface_hovered)
            .with_focused(self.surface_hovered)
            .with_pressed(self.session_tab_highlight);
        let selected = ButtonBackgrounds::new(self.session_tab_highlight)
            .with_hovered(self.session_tab_highlight)
            .with_focused(self.session_tab_highlight)
            .with_pressed(self.session_tab_highlight);
        ButtonStyle::new(backgrounds, TextStyle::new(11.0, self.text))
            .with_selected_backgrounds(selected)
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_padding(Edges::uniform(4.0))
            .with_icon_size(16.0)
    }

    pub fn navigation_button_style(&self) -> ButtonStyle {
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
