use super::*;
#[test]
fn cancelling_a_wait_stops_the_worker_without_publishing_a_completed_item() {
    let state = Arc::new(extension_api::ExtensionState::default());
    let items = Arc::new(ExtensionItemStore::new(state));
    let clock = Clock(items.clone());
    let definition = clock.definition();
    let binding = tools::ToolBinding::new(
        tools::ToolRegistryGeneration::new(1),
        tools::ToolBindingId::new("sleep@1").unwrap(),
        definition.name().clone(),
        definition.digest(),
        tools::ToolRuntimeKey::new("clock").unwrap(),
    );
    let source = async_utils::CancellationSource::new();
    let session = protocol::SessionId::new("s").unwrap();
    let thread = protocol::ThreadId::new("t").unwrap();
    let invocation = ToolInvocation::new(
        tools::ToolOperationId::new("wait").unwrap(),
        protocol::ToolCallId::new("call").unwrap(),
        protocol::TurnId::new("turn").unwrap(),
        binding,
        ToolPayload::FunctionArguments(json!({"duration_ms":60000})),
        tools::ToolExecutionContext::new(
            tools::EnvId::new("clock").unwrap(),
            source.token(),
            tools::ToolRuntimeAuthority::Unrestricted,
        )
        .with_session_id(session.clone())
        .with_thread_id(thread.clone()),
    );
    let worker = std::thread::spawn(move || pollster::block_on(clock.execute(invocation)));
    std::thread::sleep(Duration::from_millis(10));
    let cancelled = Instant::now();
    source.cancel();
    let result = worker.join().unwrap();
    assert!(cancelled.elapsed() < Duration::from_secs(1));
    assert!(
        matches!(result,ToolExecutionOutcome::Returned(output) if output.status()==tools::ToolOutputStatus::Error)
    );
    let result = extension_api::ItemContributor::contribute(
        items.as_ref(),
        extension_api::ThreadContext {
            session_id: &session,
            thread_id: &thread,
            sequence: 1,
        },
    )
    .unwrap();
    assert!(result.is_empty());
}
