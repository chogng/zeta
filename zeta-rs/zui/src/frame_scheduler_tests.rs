use super::{FrameInvalidation, FrameSchedule, FrameScheduler};

#[test]
fn scheduler_requests_one_platform_frame_for_repeated_work() {
    let mut scheduler = FrameScheduler::default();

    assert_eq!(
        scheduler.request(FrameInvalidation::Rebuild),
        FrameSchedule::RequestFrame
    );
    assert_eq!(
        scheduler.request(FrameInvalidation::Rebuild),
        FrameSchedule::Coalesced
    );
    assert_eq!(scheduler.take(), Some(FrameInvalidation::Rebuild));
    assert_eq!(scheduler.take(), None);
}

#[test]
fn scheduler_retains_the_strongest_invalidation() {
    let mut scheduler = FrameScheduler::default();

    scheduler.request(FrameInvalidation::Render);
    scheduler.request(FrameInvalidation::Fragment);
    scheduler.request(FrameInvalidation::Rebuild);
    scheduler.request(FrameInvalidation::Render);

    assert_eq!(scheduler.pending(), Some(FrameInvalidation::Rebuild));
}

#[test]
fn fragment_work_subsumes_render_without_forcing_a_full_rebuild() {
    let mut scheduler = FrameScheduler::default();

    scheduler.request(FrameInvalidation::Render);
    scheduler.request(FrameInvalidation::Fragment);

    assert_eq!(scheduler.take(), Some(FrameInvalidation::Fragment));
}

#[test]
fn synchronous_completion_clears_pending_work() {
    let mut scheduler = FrameScheduler::default();
    scheduler.request(FrameInvalidation::Rebuild);

    scheduler.clear();

    assert_eq!(scheduler.pending(), None);
    assert_eq!(
        scheduler.request(FrameInvalidation::Render),
        FrameSchedule::RequestFrame
    );
}
