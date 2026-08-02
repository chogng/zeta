/// Work that must be completed before the next frame can be presented.
///
/// Variants are ordered by cost and subsumption: rebuilding presentation also produces the scene
/// needed for rendering, so it supersedes a render-only request.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FrameInvalidation {
    /// Render the current scene without rebuilding presentation.
    Render,
    /// Rebuild presentation and then render the resulting scene.
    Rebuild,
}

/// Whether a host must wake its platform event loop after scheduling frame work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameSchedule {
    /// No frame was pending, so the host must request one from the platform.
    RequestFrame,
    /// An existing frame request now owns the additional work.
    Coalesced,
}

/// Coalesces invalidation produced between platform frames.
///
/// Hosts call [`FrameScheduler::request`] when state changes and wake the platform only for
/// [`FrameSchedule::RequestFrame`]. At the start of a redraw callback, the host consumes the
/// strongest pending invalidation with [`FrameScheduler::take`]. Requests arriving before that
/// callback are therefore reduced to one frame without zui depending on a windowing backend.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FrameScheduler {
    pending: Option<FrameInvalidation>,
}

impl FrameScheduler {
    /// Adds frame work and reports whether the platform needs a new frame request.
    pub fn request(&mut self, invalidation: FrameInvalidation) -> FrameSchedule {
        let schedule = if self.pending.is_some() {
            FrameSchedule::Coalesced
        } else {
            FrameSchedule::RequestFrame
        };
        self.pending = Some(
            self.pending
                .map_or(invalidation, |pending| pending.max(invalidation)),
        );
        schedule
    }

    /// Consumes the strongest work requested for the next frame.
    pub fn take(&mut self) -> Option<FrameInvalidation> {
        self.pending.take()
    }

    /// Discards pending work after a host completed an equivalent rebuild synchronously.
    pub fn clear(&mut self) {
        self.pending = None;
    }

    pub const fn pending(&self) -> Option<FrameInvalidation> {
        self.pending
    }
}

#[cfg(test)]
#[path = "frame_scheduler_tests.rs"]
mod tests;
