use super::schedule_command;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::completion::Completion;
use crate::app::requests::RequestKey;
use crate::app::requests::RequestTasks;
use crate::host::Command as HostCommand;
use crate::keymap::Command as KeymapCommand;
use crate::theme::Command as ThemeCommand;
use crate::thread::Command as ThreadCommand;
use std::collections::VecDeque;

#[test]
fn unrelated_actions_bypass_a_busy_request_without_losing_same_domain_order() {
    let mut app = App::new();
    let mut requests = RequestTasks::default();
    let (release, wait) = std::sync::mpsc::sync_channel(0);
    requests.spawn(
        Some(RequestKey::Config),
        "zeta-tui-test-write",
        move || {
            wait.recv().expect("the test releases the write request");
            Completion::Presentation(Err("finished".into()))
        },
        &mut app,
    );
    let mut queued = VecDeque::new();
    let write = ThemeCommand::Set {
        preference: "zeta-code-dark".into(),
    }
    .into();

    assert!(schedule_command(Some(write), &requests, &mut queued).is_none());
    assert!(matches!(
        schedule_command(
            Some(KeymapCommand::OpenEditor.into()),
            &requests,
            &mut queued
        ),
        Some(AppCommand::Keymap(KeymapCommand::OpenEditor))
    ));
    assert!(matches!(
        schedule_command(
            Some(ThreadCommand::Interrupt.into()),
            &requests,
            &mut queued
        ),
        Some(AppCommand::Thread(ThreadCommand::Interrupt))
    ));
    assert_eq!(queued.len(), 1);
    release
        .send(())
        .expect("the write request remains alive until released");
    let completed = (0..10_000)
        .find_map(|_| {
            let completed = requests.poll();
            if completed.is_empty() {
                std::thread::yield_now();
                None
            } else {
                Some(completed)
            }
        })
        .expect("the released write request completes");
    assert_eq!(completed.len(), 1);
    assert!(matches!(
        schedule_command(None, &requests, &mut queued),
        Some(AppCommand::Theme(ThemeCommand::Set { .. }))
    ));
}

#[test]
fn interrupt_bypasses_an_active_interaction_response() {
    let mut app = App::new();
    let mut requests = RequestTasks::default();
    let (release, wait) = std::sync::mpsc::sync_channel(0);
    requests.spawn(
        Some(RequestKey::Interaction),
        "zeta-tui-test-interaction",
        move || {
            wait.recv()
                .expect("the test releases the interaction request");
            Completion::Presentation(Err("finished".into()))
        },
        &mut app,
    );
    let mut queued = VecDeque::new();

    assert!(matches!(
        schedule_command(
            Some(ThreadCommand::Interrupt.into()),
            &requests,
            &mut queued
        ),
        Some(AppCommand::Thread(ThreadCommand::Interrupt))
    ));
    release
        .send(())
        .expect("the interaction request remains alive until released");
    let completed = (0..10_000)
        .find_map(|_| {
            let completed = requests.poll();
            if completed.is_empty() {
                std::thread::yield_now();
                None
            } else {
                Some(completed)
            }
        })
        .expect("the released interaction request completes");
    assert_eq!(completed.len(), 1);
}

#[test]
fn quit_bypasses_a_pending_request() {
    let mut app = App::new();
    let mut requests = RequestTasks::default();
    requests.spawn(
        Some(RequestKey::Config),
        "zeta-tui-test-write",
        || Completion::Presentation(Err("finished".into())),
        &mut app,
    );
    let mut queued = VecDeque::new();

    assert!(matches!(
        schedule_command(Some(AppCommand::Quit), &requests, &mut queued),
        Some(AppCommand::Quit)
    ));
    assert!(queued.is_empty());
}

#[test]
fn repeated_clipboard_availability_refreshes_are_coalesced() {
    let requests = RequestTasks::default();
    let mut queued = VecDeque::from([AppCommand::from(
        HostCommand::RefreshClipboardImageAvailability,
    )]);

    let action = schedule_command(
        Some(HostCommand::RefreshClipboardImageAvailability.into()),
        &requests,
        &mut queued,
    );

    assert_eq!(
        action,
        Some(AppCommand::Host(
            HostCommand::RefreshClipboardImageAvailability
        ))
    );
    assert!(queued.is_empty());
}

#[test]
fn repeated_older_history_requests_are_coalesced() {
    let requests = RequestTasks::default();
    let mut queued = VecDeque::from([AppCommand::from(ThreadCommand::LoadOlderHistory)]);

    let action = schedule_command(
        Some(ThreadCommand::LoadOlderHistory.into()),
        &requests,
        &mut queued,
    );

    assert_eq!(
        action,
        Some(AppCommand::Thread(ThreadCommand::LoadOlderHistory))
    );
    assert!(queued.is_empty());
}
