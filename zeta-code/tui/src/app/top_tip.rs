use crate::render::RenderContext;
use crate::widgets::key_hint;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::time::Duration;
use std::time::Instant;

const TRANSIENT_TIP_DURATION: Duration = Duration::from_secs(5);
const POLICY_TIP: &str = "shift+tab to cycle policy";
const CLIPBOARD_IMAGE_TIP: &str = "image in clipboard · ctrl+v to paste";

#[derive(Debug)]
pub(crate) struct TopTip {
    phase: TopTipPhase,
    notice: Option<Notice>,
    clipboard_image_expires_at: Option<Instant>,
}

#[derive(Debug)]
enum TopTipPhase {
    Navigation,
    Policy { expires_at: Instant },
    Hidden,
}

#[derive(Debug)]
struct Notice {
    text: String,
    expires_at: Instant,
}

impl TopTip {
    pub(crate) fn new() -> Self {
        Self {
            phase: TopTipPhase::Navigation,
            notice: None,
            clipboard_image_expires_at: None,
        }
    }

    pub(crate) fn show_policy_tip(&mut self, now: Instant) {
        self.phase = TopTipPhase::Policy {
            expires_at: now + TRANSIENT_TIP_DURATION,
        };
    }

    pub(crate) fn hide_navigation(&mut self) {
        if matches!(self.phase, TopTipPhase::Navigation) {
            self.phase = TopTipPhase::Hidden;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.phase = TopTipPhase::Navigation;
    }

    pub(crate) fn show_notice(&mut self, text: String, now: Instant) {
        self.notice = Some(Notice {
            text,
            expires_at: now + TRANSIENT_TIP_DURATION,
        });
    }

    pub(crate) fn show_clipboard_image(&mut self, now: Instant) {
        self.clipboard_image_expires_at = Some(now + TRANSIENT_TIP_DURATION);
    }

    pub(crate) fn hide_clipboard_image(&mut self) {
        self.clipboard_image_expires_at = None;
    }

    pub(crate) fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        tip: Option<&str>,
        context: RenderContext<'_>,
    ) {
        if let Some(text) = self.text(tip) {
            key_hint::draw_right(frame, area, text, context);
        }
    }

    pub(crate) fn poll(&mut self, now: Instant) -> bool {
        let notice_expired = self
            .notice
            .as_ref()
            .is_some_and(|notice| notice.expires_at <= now);
        if notice_expired {
            self.notice = None;
        }
        let clipboard_image_expired = self
            .clipboard_image_expires_at
            .is_some_and(|expires_at| expires_at <= now);
        if clipboard_image_expired {
            self.clipboard_image_expires_at = None;
        }
        let policy_expired = matches!(
            self.phase,
            TopTipPhase::Policy { expires_at } if expires_at <= now
        );
        if policy_expired {
            self.phase = TopTipPhase::Hidden;
        }
        notice_expired || clipboard_image_expired || policy_expired
    }

    fn text<'a>(&'a self, tip: Option<&'a str>) -> Option<&'a str> {
        if let Some(notice) = self.notice.as_ref() {
            return Some(notice.text.as_str());
        }
        if self.clipboard_image_expires_at.is_some() {
            return Some(CLIPBOARD_IMAGE_TIP);
        }
        match self.phase {
            TopTipPhase::Navigation => tip,
            TopTipPhase::Policy { .. } => Some(POLICY_TIP),
            TopTipPhase::Hidden => None,
        }
    }
}

#[cfg(test)]
#[path = "top_tip_tests.rs"]
mod tests;
