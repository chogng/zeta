use ratatui::layout::Rect;

const MIN_CHAT_HISTORY_HEIGHT: u16 = 4;
const FOOTER_HEIGHT: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChatWidgetAreas {
    pub(crate) chat_history: Rect,
    pub(crate) chat_input_area: Rect,
    pub(crate) footer: Rect,
}

pub(crate) fn areas(area: Rect, chat_input_area_desired_height: u16) -> ChatWidgetAreas {
    let footer_height = FOOTER_HEIGHT.min(area.height);
    let available_above_footer = area.height.saturating_sub(footer_height);
    let chat_history_height = MIN_CHAT_HISTORY_HEIGHT.min(available_above_footer);
    let chat_input_area_height = chat_input_area_desired_height
        .min(available_above_footer.saturating_sub(chat_history_height));
    let bottom = area.y.saturating_add(area.height);
    let footer_y = bottom.saturating_sub(footer_height);
    let chat_input_area_y = footer_y.saturating_sub(chat_input_area_height);

    ChatWidgetAreas {
        chat_history: Rect {
            height: chat_input_area_y.saturating_sub(area.y),
            ..area
        },
        chat_input_area: Rect {
            y: chat_input_area_y,
            height: chat_input_area_height,
            ..area
        },
        footer: Rect {
            y: footer_y,
            height: footer_height,
            ..area
        },
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
