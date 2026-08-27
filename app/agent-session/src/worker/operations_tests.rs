use std::sync::Arc;
use std::sync::Mutex;

use zeta_app_server_protocol::protocol::session::SessionSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadProjection;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadOrigin;
use zeta_protocol::ThreadStatus;

use super::publish_subscription;
use super::workspace_title;
use crate::AgentSessionEvent;
use crate::AgentSessionEventSink;

#[test]
fn subscription_publishes_the_authoritative_thread_snapshot() {
    let session_id = SessionId::new("session-1").unwrap();
    let thread_id = ThreadId::new("thread-1").unwrap();
    let subscription = SessionSubscribeResult {
        session: Session {
            session_id: session_id.clone(),
            title: "Workspace".to_owned(),
            status: SessionStatus::Active,
            model: None,
            workspace: None,
            sequence: 3,
            threads: vec![SessionThread {
                thread_id: thread_id.clone(),
                origin: ThreadOrigin::Root,
                status: SessionThreadStatus::Active,
            }],
        },
        updates: Vec::new(),
        thread_projections: vec![SessionThreadProjection {
            thread: Thread {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                title: "Agent".to_owned(),
                status: ThreadStatus::Active,
                sequence: 7,
                usage: Default::default(),
                goal: None,
                turns: Vec::new(),
            },
            transcript: ThreadTranscriptSnapshot {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                durable_sequence: 7,
                entries: Vec::new(),
            },
            updates: Vec::new(),
        }],
        agent_tree: Default::default(),
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink_events = Arc::clone(&events);
    let sink: AgentSessionEventSink = Arc::new(move |event| {
        sink_events.lock().unwrap().push(event);
        Ok(())
    });

    publish_subscription(&sink, &subscription, &thread_id, None).unwrap();

    let events = events.lock().unwrap();
    assert!(matches!(
        events.as_slice(),
        [AgentSessionEvent::Snapshot {
            session,
            thread,
            switch_id: None,
        }] if session.session_id == session_id && thread.thread_id == thread_id && thread.sequence == 7
    ));
}

#[test]
fn workspace_title_uses_the_last_component_and_root_fallback() {
    assert_eq!(workspace_title(std::path::Path::new("/work/zeta")), "zeta");
    assert_eq!(workspace_title(std::path::Path::new("/")), "Agent Session");
}
