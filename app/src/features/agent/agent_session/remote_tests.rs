use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use super::AGENT_UNAVAILABLE_COMMAND_ERROR;
use super::AgentSessionCommand;
use super::MAX_RECONNECT_DELAY;
use super::RECONNECT_WINDOW;
// Remote Agent recovery contract tests.

use super::reconnect_delay;
use super::reconnect_delay_within_window;
use super::reject_disconnected_command;

#[test]
fn remote_reconnect_backoff_is_bounded() {
    assert_eq!(reconnect_delay(0), Duration::from_millis(250));
    assert_eq!(reconnect_delay(1), Duration::from_millis(500));
    assert_eq!(reconnect_delay(3), Duration::from_secs(2));
    assert_eq!(reconnect_delay(32), MAX_RECONNECT_DELAY);
}

#[test]
fn remote_reconnect_never_schedules_past_its_recovery_window() {
    assert_eq!(
        reconnect_delay_within_window(Duration::ZERO, 0),
        Some(Duration::from_millis(250))
    );
    assert_eq!(
        reconnect_delay_within_window(RECONNECT_WINDOW - Duration::from_secs(2), 8),
        Some(Duration::from_secs(2))
    );
    assert_eq!(
        reconnect_delay_within_window(RECONNECT_WINDOW - Duration::from_millis(100), 0),
        None
    );
    assert_eq!(reconnect_delay_within_window(RECONNECT_WINDOW, 0), None);
}

#[test]
fn remote_reconnect_rejects_queued_commands_instead_of_replaying_them() {
    let (response, result) = mpsc::sync_channel(1);
    assert!(!reject_disconnected_command(
        AgentSessionCommand::ReadDirectory {
            path: PathBuf::from("src"),
            response,
        }
    ));
    assert_eq!(
        result.recv().unwrap(),
        Err(AGENT_UNAVAILABLE_COMMAND_ERROR.to_owned())
    );
    assert!(reject_disconnected_command(AgentSessionCommand::Shutdown));
}
