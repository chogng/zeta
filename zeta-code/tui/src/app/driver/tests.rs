use super::IssueContext;
use super::schedule_command;
use crate::app::App;
use crate::app::AppCommand;
use crate::app::completion::Completion;
use crate::app::requests::RequestKey;
use crate::app::requests::RequestTasks;
use crate::host::Command as HostCommand;
use crate::keymap_setup::Command as KeymapCommand;
use crate::theme::Command as ThemeCommand;
use crate::thread::Command as ThreadCommand;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

#[test]
fn failed_issue_context_read_waits_before_retry_and_stops_after_success() {
    let thread = ThreadId::new("current").unwrap();
    let mut context = IssueContext::default();
    let now = Instant::now();
    assert!(context.should_load(&thread, now));

    // Even a slow RPC failure must wait a full second after completion.
    let failed_at = now + Duration::from_secs(5);
    let failed = context.complete(
        &thread,
        thread.clone(),
        Err("temporary disconnect".into()),
        failed_at,
    );
    assert!(matches!(failed, Some(Err(error)) if error == "temporary disconnect"));
    assert!(!context.should_load(&thread, failed_at + Duration::from_millis(999)));
    let retry_at = failed_at + Duration::from_secs(1);
    assert!(context.should_load(&thread, retry_at));

    let succeeded = context.complete(
        &thread,
        thread.clone(),
        Ok(crate::issues::Event::ContextReceived {
            session_id: SessionId::new("session").unwrap(),
            numbers: vec![3, 5],
        }),
        retry_at,
    );
    assert!(matches!(
        succeeded,
        Some(Ok(crate::issues::Event::ContextReceived { numbers, .. })) if numbers == vec![3, 5]
    ));
    assert!(!context.should_load(&thread, retry_at + Duration::from_secs(300)));
}

#[test]
fn persistent_issue_context_failures_back_off_and_report_only_once() {
    let thread = ThreadId::new("current").unwrap();
    let mut context = IssueContext::default();
    let started = Instant::now();
    let mut requests = Vec::new();
    let mut app = App::new();

    // Exercise two minutes of terminal ticks with immediately failing RPCs.
    for millis in (0..=120_000).step_by(25) {
        let now = started + Duration::from_millis(millis);
        if context.should_load(&thread, now) {
            requests.push(millis);
            if let Some(Err(error)) = context.complete(
                &thread,
                thread.clone(),
                Err(format!("failure at {millis}")),
                now,
            ) {
                app.update(crate::thread::Event::FailureReported(error));
            }
        }
    }

    assert_eq!(
        requests,
        vec![0, 1_000, 3_000, 7_000, 15_000, 31_000, 61_000, 91_000]
    );
    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].text(), "failure at 0");

    let recovered_at = started + Duration::from_secs(121);
    assert!(context.should_load(&thread, recovered_at));
    assert!(matches!(
        context.complete(
            &thread,
            thread.clone(),
            Ok(crate::issues::Event::ContextReceived {
                session_id: SessionId::new("session").unwrap(),
                numbers: vec![3],
            }),
            recovered_at,
        ),
        Some(Ok(_))
    ));
    assert!(!context.should_load(&thread, recovered_at + Duration::from_secs(300)));
}

#[test]
fn switching_issue_context_resets_backoff_and_ignores_stale_completions() {
    let previous = ThreadId::new("previous").unwrap();
    let current = ThreadId::new("current").unwrap();
    let mut context = IssueContext::default();
    let now = Instant::now();
    assert!(context.should_load(&previous, now));
    assert!(
        context
            .complete(&previous, previous.clone(), Err("failed".into()), now)
            .is_some()
    );
    assert!(!context.should_load(&previous, now));
    assert!(context.should_load(&current, now));

    for result in [
        Ok(crate::issues::Event::ContextReceived {
            session_id: SessionId::new("previous").unwrap(),
            numbers: vec![3],
        }),
        Err("late failure".into()),
    ] {
        assert!(
            context
                .complete(&current, previous.clone(), result, now)
                .is_none()
        );
        assert!(context.should_load(&current, now));
    }
    // The new thread gets its own first failure and initial retry delay.
    assert!(
        context
            .complete(&current, current.clone(), Err("current failed".into()), now)
            .is_some()
    );
    assert!(!context.should_load(&current, now + Duration::from_millis(999)));
    assert!(context.should_load(&current, now + Duration::from_secs(1)));
}

#[test]
fn empty_issue_context_is_loaded_until_the_thread_changes() {
    let first = ThreadId::new("first").unwrap();
    let second = ThreadId::new("second").unwrap();
    let mut context = IssueContext::default();
    let now = Instant::now();
    for thread in [&first, &second, &first] {
        assert!(context.should_load(thread, now));
        assert!(matches!(
            context.complete(
                thread,
                thread.clone(),
                Ok(crate::issues::Event::ContextReceived {
                    session_id: SessionId::new("session").unwrap(),
                    numbers: Vec::new(),
                }),
                now,
            ),
            Some(Ok(_))
        ));
        assert!(!context.should_load(thread, now + Duration::from_secs(300)));
    }
}

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
