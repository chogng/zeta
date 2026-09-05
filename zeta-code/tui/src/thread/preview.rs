use super::ThreadPresentationEvent;
use super::ThreadState;
use super::transcript::ChatHistoryRenderCache;
use super::transcript::ChatHistoryScroll;
use super::transcript::Message;
use super::transcript::TranscriptScrollDirection;
use super::transcript::first_scroll_target;
use super::transcript::scroll_target;
use crate::render::RenderContext;
use ratatui::layout::Rect;
use std::collections::BTreeSet;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadReadResult;
use zeta_app_server_protocol::protocol::session::ThreadHistoryBoundary;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;

/// A read-only conversation snapshot with its own history, scroll, and request lifecycle.
#[derive(Debug)]
pub(crate) struct ConversationPreview {
    pub(crate) generation: u64,
    pub(crate) title: String,
    pub(crate) scroll: ChatHistoryScroll,
    pub(crate) cache: ChatHistoryRenderCache,
    request: SessionThreadReadParams,
    thread: ThreadState,
    boundary: Option<ThreadHistoryBoundary>,
    loading: bool,
    error: Option<String>,
}

impl ConversationPreview {
    pub(crate) fn session_id(&self) -> &zeta_protocol::SessionId {
        &self.request.session_id
    }

    pub(crate) fn new(generation: u64, title: String, request: SessionThreadReadParams) -> Self {
        Self {
            generation,
            title,
            request,
            scroll: Default::default(),
            cache: Default::default(),
            thread: Default::default(),
            boundary: None,
            loading: true,
            error: None,
        }
    }

    pub(crate) fn messages(&self) -> Vec<Message> {
        self.thread.views(&BTreeSet::new(), None)
    }

    pub(crate) fn notice(&self) -> Option<&str> {
        if self.loading {
            Some("Loading conversation…")
        } else {
            self.error.as_deref()
        }
    }

    pub(crate) fn install(&mut self, result: Result<SessionThreadReadResult, String>) {
        self.loading = false;
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        if result.thread.session_id != self.request.session_id
            || result.thread.thread_id != self.request.thread_id
            || result.transcript.session_id != self.request.session_id
            || result.transcript.thread_id != self.request.thread_id
        {
            self.error = Some("Preview response belongs to another conversation".into());
            return;
        }
        let Some(boundary) = result.history else {
            self.error = Some("Preview response is missing its history boundary".into());
            return;
        };
        self.error = None;
        self.boundary = Some(boundary);
        if matches!(
            self.request.history,
            Some(ThreadSnapshotHistory::Before { .. })
        ) {
            self.thread
                .update(ThreadPresentationEvent::TranscriptHistoryPageReceived(
                    result.transcript,
                ));
        } else {
            self.thread
                .update(ThreadPresentationEvent::TranscriptSnapshotReceived(
                    result.transcript,
                ));
        }
    }

    pub(crate) fn navigate(
        &mut self,
        direction: TranscriptScrollDirection,
        area: Rect,
        header_rows: usize,
        context: RenderContext<'_>,
    ) -> Option<SessionThreadReadParams> {
        let messages = self.messages();
        if let Some(target) = scroll_target(
            area,
            header_rows,
            &messages,
            &self.scroll,
            &self.cache,
            context,
            direction,
        ) && self.scroll.apply(target)
        {
            return None;
        }
        if direction == TranscriptScrollDirection::Down || self.loading {
            return None;
        }
        self.older()
    }

    pub(crate) fn first(&mut self) -> Option<SessionThreadReadParams> {
        if let Some(target) = first_scroll_target(true, &self.messages()) {
            self.scroll.apply(target);
        }
        if self.loading {
            return None;
        }
        self.older()
    }

    fn older(&mut self) -> Option<SessionThreadReadParams> {
        let boundary = self.boundary.as_ref()?;
        if !boundary.has_older_turns {
            return None;
        }
        self.request.history = Some(ThreadSnapshotHistory::Before {
            turn_id: boundary.oldest_turn_id.clone()?,
            turn_limit: 50,
        });
        self.loading = true;
        Some(self.request.clone())
    }
}
