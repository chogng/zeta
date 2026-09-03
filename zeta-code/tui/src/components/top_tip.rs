use super::key_hint;
use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::time::Duration;
use std::time::Instant;

const NOTICE_DURATION: Duration = Duration::from_secs(3);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub(crate) struct TopTip {
    index: usize,
    refresh_at: Instant,
    notice: Option<Notice>,
}

#[derive(Debug)]
struct Notice {
    text: String,
    expires_at: Instant,
}

impl TopTip {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            index: 0,
            refresh_at: now + REFRESH_INTERVAL,
            notice: None,
        }
    }

    pub(crate) fn show_notice(&mut self, text: String, now: Instant) {
        self.notice = Some(Notice {
            text,
            expires_at: now + NOTICE_DURATION,
        });
    }

    pub(crate) fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        tips: &[Option<&str>],
        context: RenderContext<'_>,
    ) {
        if let Some(text) = self.text(tips) {
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
        if now < self.refresh_at {
            return notice_expired;
        }
        self.index = self.index.wrapping_add(1);
        self.refresh_at = now + REFRESH_INTERVAL;
        notice_expired || self.notice.is_none()
    }

    fn text<'a>(&'a self, tips: &[Option<&'a str>]) -> Option<&'a str> {
        if let Some(notice) = self.notice.as_ref() {
            return Some(notice.text.as_str());
        }
        let count = tips.iter().flatten().count();
        tips.iter()
            .filter_map(|tip| *tip)
            .nth(self.index % count.max(1))
    }
}

#[cfg(test)]
#[path = "top_tip_tests.rs"]
mod tests;
