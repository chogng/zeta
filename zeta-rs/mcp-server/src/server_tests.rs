use super::*;
use crate::agent::{AgentOutcomeStatus, ReplyAgentRequest, StartAgentRequest};
use crate::events::{AgentEvents, AgentProgress, InteractionResolution};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
use zeta_protocol::{
    ActionApprovalCapability, ActionApprovalCapabilityKind, ActionApprovalDecision,
    ActionApprovalRequest, AgentRequest, AgentRequestEnvelope, AgentResponse, RequestId, SessionId,
    ThreadId, TurnId, TurnInteraction,
};

struct FakeAgent;

impl AgentService for FakeAgent {
    fn start(
        &self,
        request: StartAgentRequest,
        _: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        Ok(outcome(
            &request.invocation_id,
            AgentOutcomeStatus::Completed,
        ))
    }

    fn reply(
        &self,
        request: ReplyAgentRequest,
        _: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        Ok(outcome(
            &request.invocation_id,
            AgentOutcomeStatus::Completed,
        ))
    }
}

struct BlockingAgent {
    started: AtomicBool,
}

struct ProgressAgent;

impl AgentService for ProgressAgent {
    fn start(
        &self,
        request: StartAgentRequest,
        _: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        events.progress(AgentProgress {
            message: "Turn started".into(),
        });
        Ok(outcome(
            &request.invocation_id,
            AgentOutcomeStatus::Completed,
        ))
    }

    fn reply(
        &self,
        _: ReplyAgentRequest,
        _: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        unreachable!()
    }
}

struct InteractionAgent;

impl AgentService for InteractionAgent {
    fn start(
        &self,
        request: StartAgentRequest,
        _: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        let envelope = approval_envelope();
        let status = match events.resolve_interaction(&envelope) {
            InteractionResolution::Respond(AgentResponse::Approval { response })
                if response.decision == ActionApprovalDecision::ApproveOnce =>
            {
                AgentOutcomeStatus::Completed
            }
            _ => AgentOutcomeStatus::WaitingForApproval,
        };
        Ok(outcome(&request.invocation_id, status))
    }

    fn reply(
        &self,
        _: ReplyAgentRequest,
        _: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        unreachable!()
    }
}

impl AgentService for BlockingAgent {
    fn start(
        &self,
        request: StartAgentRequest,
        cancellation: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        self.started.store(true, Ordering::Release);
        while !cancellation.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
        }
        Ok(outcome(
            &request.invocation_id,
            AgentOutcomeStatus::Interrupted,
        ))
    }

    fn reply(
        &self,
        _: ReplyAgentRequest,
        _: &AtomicBool,
        _: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        unreachable!()
    }
}

fn outcome(invocation_id: &str, status: AgentOutcomeStatus) -> AgentOutcome {
    AgentOutcome {
        invocation_id: invocation_id.into(),
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        turn_id: TurnId::new("turn-1").unwrap(),
        status,
        content: "done".into(),
    }
}

fn initialize(server: &McpServer) {
    let response = server
        .handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"test","version":"1"},"capabilities":{}}}"#,
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&response).unwrap()["result"]["serverInfo"]["name"],
        "zeta-mcp-server"
    );
}

#[test]
fn initialize_and_list_tools_advertise_agent_surface() {
    let server = McpServer::new(Arc::new(FakeAgent));
    initialize(&server);

    let response = server
        .handle_line(r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list","params":{}}"#)
        .unwrap();
    let response: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response["id"], "tools");
    assert_eq!(response["result"]["tools"][0]["name"], TOOL_START);
    assert_eq!(response["result"]["tools"][1]["name"], TOOL_REPLY);
}

#[test]
fn tool_call_returns_structured_zeta_identity() {
    let server = McpServer::new(Arc::new(FakeAgent));
    initialize(&server);

    let response = server
        .handle_line(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"zeta","arguments":{"invocationId":"call-1","prompt":"inspect"}}}"#,
        )
        .unwrap();
    let response: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response["result"]["isError"], false);
    assert_eq!(
        response["result"]["structuredContent"]["invocationId"],
        "call-1"
    );
    assert_eq!(
        response["result"]["structuredContent"]["threadId"],
        "thread-1"
    );
}

#[test]
fn progress_uses_the_callers_exact_progress_token() {
    let server = McpServer::new(Arc::new(ProgressAgent));
    initialize(&server);
    let (outgoing, incoming) = mpsc::channel();
    let response = server
        .handle_line_with_outgoing(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"zeta","_meta":{"progressToken":"progress-1"},"arguments":{"invocationId":"progress-call","prompt":"inspect"}}}"#,
            outgoing,
        )
        .unwrap();
    let notification: Value =
        serde_json::from_str(&incoming.recv_timeout(Duration::from_secs(1)).unwrap()).unwrap();

    assert_eq!(notification["method"], "notifications/progress");
    assert_eq!(notification["params"]["progressToken"], "progress-1");
    assert_eq!(
        serde_json::from_str::<Value>(&response).unwrap()["result"]["isError"],
        false
    );
}

#[test]
fn elicitation_response_is_bound_to_the_exact_agent_interaction() {
    let server = McpServer::new(Arc::new(InteractionAgent));
    let initialized = server
        .handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"test","version":"1"},"capabilities":{"elicitation":{"form":{}}}}}"#,
        )
        .unwrap();
    assert!(initialized.contains("zeta-mcp-server"));
    let (outgoing, incoming) = mpsc::channel();
    let worker_server = server.clone();
    let worker_outgoing = outgoing.clone();
    let worker = thread::spawn(move || {
        worker_server.handle_line_with_outgoing(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"zeta","arguments":{"invocationId":"interaction-call","prompt":"inspect"}}}"#,
            worker_outgoing,
        )
    });
    let elicitation: Value =
        serde_json::from_str(&incoming.recv_timeout(Duration::from_secs(1)).unwrap()).unwrap();
    assert_eq!(elicitation["method"], "elicitation/create");
    assert_eq!(
        elicitation["params"]["requestedSchema"]["properties"]["decision"]["type"],
        "string"
    );
    let response = json!({
        "jsonrpc": "2.0",
        "id": elicitation["id"],
        "result": {
            "action": "accept",
            "content": {"decision": "approveOnce"}
        }
    });
    assert!(
        server
            .handle_line_with_outgoing(&response.to_string(), outgoing)
            .is_none()
    );
    let final_response: Value = serde_json::from_str(&worker.join().unwrap().unwrap()).unwrap();
    assert_eq!(
        final_response["result"]["structuredContent"]["status"],
        "completed"
    );
}

#[test]
fn cancel_notification_reaches_active_tool_call() {
    let agent = Arc::new(BlockingAgent {
        started: AtomicBool::new(false),
    });
    let server = McpServer::new(agent.clone());
    initialize(&server);
    let worker_server = server.clone();
    let worker = thread::spawn(move || {
        worker_server.handle_line(
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"zeta","arguments":{"invocationId":"cancel-1","prompt":"wait"}}}"#,
        )
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    while !agent.started.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "tool call did not start");
        thread::sleep(Duration::from_millis(1));
    }

    assert!(
        server
            .handle_line(
                r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":9,"reason":"test"}}"#,
            )
            .is_none()
    );
    assert!(worker.join().unwrap().is_none());
}

#[test]
fn business_methods_are_gated_by_initialize() {
    let server = McpServer::new(Arc::new(FakeAgent));
    let response = server
        .handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#)
        .unwrap();
    let response: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response["error"]["code"], -32001);
}

#[test]
fn initialize_requires_client_capabilities() {
    let server = McpServer::new(Arc::new(FakeAgent));
    let response = server
        .handle_line(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","clientInfo":{"name":"test","version":"1"}}}"#,
        )
        .unwrap();
    let response: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response["error"]["code"], -32602);
}

#[test]
fn tool_output_truncation_preserves_utf8_and_byte_limit() {
    let content = "界".repeat(MAX_TOOL_RESULT_BYTES);
    let truncated = truncate_utf8(content, MAX_TOOL_RESULT_BYTES);

    assert!(truncated.len() <= MAX_TOOL_RESULT_BYTES);
    assert!(truncated.ends_with("[output truncated]"));
}

fn approval_envelope() -> AgentRequestEnvelope {
    AgentRequestEnvelope {
        session_id: SessionId::new("session-1").unwrap(),
        thread_id: ThreadId::new("thread-1").unwrap(),
        turn_id: TurnId::new("turn-1").unwrap(),
        interaction: TurnInteraction {
            request_id: RequestId::new("approval-1").unwrap(),
            item_id: None,
            request: AgentRequest::Approval {
                request: ActionApprovalRequest {
                    action_digest: "digest".into(),
                    policy_revision: "policy".into(),
                    capabilities: vec![ActionApprovalCapability {
                        kind: ActionApprovalCapabilityKind::Network,
                        scope: "api.example.com".into(),
                    }],
                    reason: "Network access is required".into(),
                    sandbox_denial: None,
                },
            },
            deadline: None,
        },
    }
}
