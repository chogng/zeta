use std::time::Duration;
use std::time::Instant;

const DISPLAY_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Default)]
pub(super) struct StatusNotice {
    visible: Option<VisibleStatusNotice>,
}

#[derive(Debug)]
struct VisibleStatusNotice {
    text: String,
    expires_at: Instant,
}

impl StatusNotice {
    pub(super) fn show(&mut self, text: String, now: Instant) {
        self.visible = Some(VisibleStatusNotice {
            text,
            expires_at: now + DISPLAY_DURATION,
        });
    }

    pub(super) fn text(&self) -> Option<&str> {
        self.visible.as_ref().map(|notice| notice.text.as_str())
    }

    pub(super) fn expire(&mut self, now: Instant) -> bool {
        if !self
            .visible
            .as_ref()
            .is_some_and(|notice| notice.expires_at <= now)
        {
            return false;
        }
        self.visible = None;
        true
    }
}

#[cfg(test)]
#[path = "status_notice_tests.rs"]
mod tests;
