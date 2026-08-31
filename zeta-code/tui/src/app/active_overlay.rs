use crate::components::detail_list::DetailList;
use crate::components::overlay::DetailOverlay;
use crate::components::overlay::OverlayInputOutcome;
use crossterm::event::KeyEvent;

#[derive(Debug)]
pub(crate) struct ActiveOverlay {
    detail: DetailOverlay,
}

impl ActiveOverlay {
    pub(crate) fn detail(detail: DetailList) -> Self {
        Self {
            detail: DetailOverlay::new(detail),
        }
    }

    pub(crate) fn detail_view(&self) -> &DetailOverlay {
        &self.detail
    }

    #[cfg(test)]
    pub(crate) fn title(&self) -> &str {
        self.detail.title()
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> OverlayInputOutcome {
        self.detail.handle_key(key)
    }
}
