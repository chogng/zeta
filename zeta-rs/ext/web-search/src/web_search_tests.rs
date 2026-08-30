use std::sync::Arc;

use zeta_async_utils::CancellationSource;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_protocol::ToolSourceProvenance;
use zeta_protocol::TurnId;
use zeta_tools::EnvId;
use zeta_tools::ToolBinding;
use zeta_tools::ToolBindingId;
use zeta_tools::ToolContent;
use zeta_tools::ToolExecutionContext;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolInvocation;
use zeta_tools::ToolOperationId;
use zeta_tools::ToolPayload;
use zeta_tools::ToolRegistryGeneration;
use zeta_tools::ToolRuntimeAuthority;
use zeta_tools::ToolRuntimeKey;

use crate::WebSearchBackend;
use crate::WebSearchError;
use crate::WebSearchRequest;
use crate::WebSearchResponse;
use crate::WebSearchResult;
use crate::install;

struct FakeBackend;

impl WebSearchBackend for FakeBackend {
    fn service_name(&self) -> &str {
        "test search"
    }

    fn network_scopes(&self) -> Vec<String> {
        vec!["search.example.com".into()]
    }

    fn credential_reference(&self) -> Option<String> {
        Some("secret:test-search".into())
    }

    fn search(
        &self,
        request: &WebSearchRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<WebSearchResponse, WebSearchError> {
        Ok(WebSearchResponse {
            results: vec![WebSearchResult {
                title: request.search_query[0].q.clone(),
                url: "https://example.com/result".into(),
                snippet: "matched".into(),
                published_at: None,
            }],
        })
    }
}

#[test]
fn install_contributes_capability_tool_and_executes_backend() {
    let mut builder = ExtensionRegistryBuilder::new();
    install(&mut builder, Arc::new(FakeBackend));
    let contribution = builder
        .build()
        .contribute_capability_tools()
        .unwrap()
        .remove(0);
    let executor = contribution.executor().clone();
    let definition = executor.definition();
    let binding = ToolBinding::new(
        ToolRegistryGeneration::new(1),
        ToolBindingId::new("web-search@1").unwrap(),
        definition.name().clone(),
        definition.digest(),
        ToolRuntimeKey::new("web-search-runtime").unwrap(),
    )
    .with_source_chain(vec![ToolSourceProvenance::Extension {
        id: "web-search".into(),
    }]);
    let cancellation = CancellationSource::new();
    let invocation = ToolInvocation::new(
        ToolOperationId::new("operation-1").unwrap(),
        zeta_protocol::ToolCallId::new("call-1").unwrap(),
        TurnId::new("turn-1").unwrap(),
        binding,
        ToolPayload::FunctionArguments(serde_json::json!({
            "search_query": [{"q": "rust extension design"}],
            "response_length": "short"
        })),
        ToolExecutionContext::new(
            EnvId::new("host-extension").unwrap(),
            cancellation.token(),
            ToolRuntimeAuthority::Unrestricted,
        ),
    );

    let outcome = pollster::block_on(executor.execute(invocation));

    assert!(matches!(
        outcome,
        ToolExecutionOutcome::Returned(output)
            if matches!(&output.content()[0], ToolContent::Text(text) if text.contains("rust extension design"))
    ));
}
