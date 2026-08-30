use super::*;
use crate::events::IgnoreAgentEvents;
use crate::events::{AgentEvents, AgentProgress, InteractionResolution};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use zeta_app_server::AppServer;
use zeta_app_server_client::InProcessTransport;
use zeta_app_server_protocol::protocol::common::{ClientCapabilities, ClientInfo};
use zeta_app_server_protocol::protocol::initialize::InitializeParams;
use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, InMemoryThreadStore, ModelService, ThreadController};
use zeta_protocol::{
    ContentPart, InputItem as ModelInputItem, ModelRequest, ModelResponse, ResponseItem, StopReason,
};

struct EchoModel {
    invocations: Arc<AtomicUsize>,
}

impl ModelService for EchoModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        let prompt = request
            .input
            .iter()
            .rev()
            .find_map(|item| match item {
                ModelInputItem::Message(message) => {
                    message.content.iter().find_map(|content| match content {
                        ContentPart::Text(text) => Some(text.as_str()),
                        ContentPart::ImageUrl { .. } | ContentPart::ImageAttachment { .. } => None,
                    })
                }
                ModelInputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("Zeta: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

fn service() -> (AppServerAgentService<InProcessTransport>, Arc<AtomicUsize>) {
    let invocations = Arc::new(AtomicUsize::new(0));
    let service = service_with_model(Arc::new(EchoModel {
        invocations: invocations.clone(),
    }));
    (service, invocations)
}

fn service_with_model(model: Arc<dyn ModelService>) -> AppServerAgentService<InProcessTransport> {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let app_server = AppServer::new(threads, model);
    let mut client = AppServerClient::new(InProcessTransport::from_server(app_server));
    client
        .initialize(InitializeParams {
            client_info: ClientInfo {
                name: "mcp-server-test".into(),
                version: "1".into(),
            },
            capabilities: ClientCapabilities::default(),
        })
        .unwrap();
    AppServerAgentService::new(
        client,
        RuntimeLimits {
            default_turn_timeout: Duration::from_secs(1),
            maximum_turn_timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(1),
        },
    )
}

#[test]
fn start_and_reply_use_canonical_app_server_threads() {
    let (service, invocations) = service();
    let cancellation = AtomicBool::new(false);
    let started = service
        .start(
            StartAgentRequest {
                invocation_id: "start-1".into(),
                prompt: "first".into(),
                timeout: None,
            },
            &cancellation,
            &IgnoreAgentEvents,
        )
        .unwrap();
    let replied = service
        .reply(
            ReplyAgentRequest {
                invocation_id: "reply-1".into(),
                thread_id: started.thread_id.to_string(),
                prompt: "second".into(),
                timeout: None,
            },
            &cancellation,
            &IgnoreAgentEvents,
        )
        .unwrap();

    assert_eq!(started.status, AgentOutcomeStatus::Completed);
    assert_eq!(started.content, "Zeta: first");
    assert_eq!(replied.status, AgentOutcomeStatus::Completed);
    assert_eq!(replied.content, "Zeta: second");
    assert_eq!(replied.session_id, started.session_id);
    assert_eq!(replied.thread_id, started.thread_id);
    assert_eq!(invocations.load(Ordering::Relaxed), 2);
}

#[derive(Default)]
struct RecordingEvents {
    progress: Mutex<Vec<String>>,
}

impl AgentEvents for RecordingEvents {
    fn progress(&self, progress: AgentProgress) {
        self.progress.lock().unwrap().push(progress.message);
    }

    fn resolve_interaction(
        &self,
        _: &zeta_protocol::AgentRequestEnvelope,
    ) -> InteractionResolution {
        InteractionResolution::Unavailable
    }
}

#[test]
fn app_server_thread_updates_are_projected_as_bounded_progress() {
    let (service, _) = service();
    let events = RecordingEvents::default();
    let outcome = service
        .start(
            StartAgentRequest {
                invocation_id: "progress-real".into(),
                prompt: "work".into(),
                timeout: None,
            },
            &AtomicBool::new(false),
            &events,
        )
        .unwrap();

    assert_eq!(outcome.status, AgentOutcomeStatus::Completed);
    assert!(
        events
            .progress
            .lock()
            .unwrap()
            .iter()
            .any(|message| message == "Turn started")
    );
}

#[test]
fn completed_invocation_replays_and_conflicting_payload_is_rejected() {
    let (service, invocations) = service();
    let cancellation = AtomicBool::new(false);
    let request = StartAgentRequest {
        invocation_id: "stable-1".into(),
        prompt: "same".into(),
        timeout: None,
    };
    let first = service
        .start(request.clone(), &cancellation, &IgnoreAgentEvents)
        .unwrap();
    let replay = service
        .start(request, &cancellation, &IgnoreAgentEvents)
        .unwrap();
    let conflict = service.start(
        StartAgentRequest {
            invocation_id: "stable-1".into(),
            prompt: "different".into(),
            timeout: None,
        },
        &cancellation,
        &IgnoreAgentEvents,
    );

    assert_eq!(first, replay);
    assert_eq!(invocations.load(Ordering::Relaxed), 1);
    assert_eq!(conflict, Err(AgentCallError::InvocationConflict));
}

#[test]
fn command_identity_is_namespaced_by_principal() {
    let first = command_id("principal-a", "shared-invocation", "session").unwrap();
    let second = command_id("principal-b", "shared-invocation", "session").unwrap();

    assert_ne!(first, second);
}

#[test]
fn reply_rejects_thread_not_authorized_for_principal() {
    let (service, _) = service();
    let result = service.reply(
        ReplyAgentRequest {
            invocation_id: "reply-unknown".into(),
            thread_id: "thread-elsewhere".into(),
            prompt: "continue".into(),
            timeout: None,
        },
        &AtomicBool::new(false),
        &IgnoreAgentEvents,
    );

    assert_eq!(result, Err(AgentCallError::ThreadNotOwned));
}

struct BlockingModel {
    started: Arc<AtomicBool>,
}

impl ModelService for BlockingModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.started.store(true, Ordering::Release);
        loop {
            if cancellation.check().is_err() {
                return Err(CoreError::Cancelled("test cancellation".into()));
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[test]
fn cancellation_interrupts_the_exact_app_server_turn() {
    let started = Arc::new(AtomicBool::new(false));
    let service = Arc::new(service_with_model(Arc::new(BlockingModel {
        started: started.clone(),
    })));
    let cancellation = Arc::new(AtomicBool::new(false));
    let worker_service = service.clone();
    let worker_cancellation = cancellation.clone();
    let worker = thread::spawn(move || {
        worker_service.start(
            StartAgentRequest {
                invocation_id: "cancel-real".into(),
                prompt: "wait".into(),
                timeout: None,
            },
            &worker_cancellation,
            &IgnoreAgentEvents,
        )
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    while !started.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "model invocation did not start");
        thread::sleep(Duration::from_millis(1));
    }

    cancellation.store(true, Ordering::Release);
    let outcome = worker.join().unwrap().unwrap();

    assert_eq!(outcome.status, AgentOutcomeStatus::Interrupted);
}
