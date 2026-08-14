use std::cell::Cell;
use std::cell::RefCell;
use std::time::Duration;

use super::RECONNECT_WINDOW;
use super::ReconnectFailure;
use super::reconnect_delay;
use super::retry;

#[test]
fn transport_failures_retry_with_bounded_backoff_until_success() {
    let elapsed = Cell::new(Duration::ZERO);
    let attempts = Cell::new(0_usize);
    let reports = RefCell::new(Vec::new());

    let result = retry(
        "initial disconnect",
        || {
            let attempt = attempts.get() + 1;
            attempts.set(attempt);
            if attempt < 3 {
                Err(ReconnectFailure::Retryable(format!(
                    "transport failure {attempt}"
                )))
            } else {
                Ok("ready")
            }
        },
        |delay| elapsed.set(elapsed.get() + delay),
        || elapsed.get(),
        |attempt, delay| reports.borrow_mut().push((attempt, delay)),
    )
    .unwrap();

    assert_eq!(result, "ready");
    assert_eq!(attempts.get(), 3);
    assert_eq!(
        reports.into_inner(),
        vec![
            (1, Duration::from_millis(250)),
            (2, Duration::from_millis(500)),
            (3, Duration::from_secs(1)),
        ]
    );
}

#[test]
fn runtime_or_protocol_change_stops_without_more_retries() {
    let attempts = Cell::new(0_usize);
    let error = retry::<()>(
        "initial disconnect",
        || {
            attempts.set(attempts.get() + 1);
            Err(ReconnectFailure::Terminal("schema changed".into()))
        },
        |_| {},
        || Duration::ZERO,
        |_, _| {},
    )
    .unwrap_err();

    assert_eq!(attempts.get(), 1);
    assert_eq!(error, "schema changed");
}

#[test]
fn exhausted_window_does_not_start_an_unbounded_attempt() {
    let attempts = Cell::new(0_usize);
    let error = retry::<()>(
        "ssh stream closed",
        || {
            attempts.set(attempts.get() + 1);
            Err(ReconnectFailure::Retryable("still closed".into()))
        },
        |_| {},
        || RECONNECT_WINDOW,
        |_, _| {},
    )
    .unwrap_err();

    assert_eq!(attempts.get(), 0);
    assert!(error.contains("within 30 seconds after 0 attempts"));
    assert!(error.contains("ssh stream closed"));
    assert_eq!(reconnect_delay(10), Duration::from_secs(2));
}
