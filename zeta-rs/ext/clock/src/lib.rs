//! Cancellable model waits with structured elapsed-time results.
use extension_api::ExtensionError;
use extension_api::ExtensionItemStore;
use extension_api::ExtensionRegistryBuilder;
use extension_api::ReadOnlyToolContributor;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tools::ToolConcurrency;
use tools::ToolContent;
use tools::ToolDefinition;
use tools::ToolExecutionFuture;
use tools::ToolExecutionOutcome;
use tools::ToolExecutor;
use tools::ToolInputSchema;
use tools::ToolInvocation;
use tools::ToolLoading;
use tools::ToolName;
use tools::ToolOutput;
use tools::ToolOutputSchema;
use tools::ToolPayload;
use tools::ToolSchemaMode;
struct Clock(Arc<ExtensionItemStore>);
/// Installs a wait tool whose maximum duration is one minute per call.
pub fn install(builder: &mut ExtensionRegistryBuilder, items: Arc<ExtensionItemStore>) {
    builder.read_only_tool_contributor("clock", Arc::new(Clock(items)));
}
impl ReadOnlyToolContributor for Clock {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        Ok(vec![Arc::new(Clock(self.0.clone()))])
    }
}
impl ToolExecutor for Clock {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::function(ToolName::new("sleep").unwrap(),"Wait for up to 60 seconds. A Turn cancellation ends the wait immediately. The result reports elapsed milliseconds.",ToolInputSchema::parse(json!({"type":"object","properties":{"duration_ms":{"type":"integer","minimum":0,"maximum":60000}},"required":["duration_ms"],"additionalProperties":false})).unwrap(),ToolOutputSchema::Unspecified,ToolSchemaMode::Strict,ToolLoading::Eager).expect("valid sleep tool definition")
    }
    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::ParallelSafe
    }
    fn execute(&self, invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(async move {
            let result = (|| -> Result<Vec<ToolContent>, String> {
                let (Some(session), Some(thread)) = (
                    invocation.context().session_id(),
                    invocation.context().thread_id(),
                ) else {
                    return Err("Sleep requires a bound Session and Thread".into());
                };
                let ToolPayload::FunctionArguments(args) = invocation.payload() else {
                    return Err("Sleep requires function arguments".into());
                };
                if args.as_object().is_none_or(|args| args.len() != 1) {
                    return Err("Sleep takes only duration_ms".into());
                }
                let ms = args["duration_ms"]
                    .as_u64()
                    .filter(|ms| *ms <= 60_000)
                    .ok_or("duration_ms must be between 0 and 60000")?;
                let start = Instant::now();
                let deadline = start + Duration::from_millis(ms);
                loop {
                    invocation
                        .context()
                        .cancellation()
                        .check()
                        .map_err(|e| e.reason().to_string())?;
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    std::thread::sleep((deadline - now).min(Duration::from_millis(5)));
                }
                let elapsed = start.elapsed().as_millis();
                self.0
                    .publish(
                        session,
                        thread,
                        extension_items::ExtensionItem {
                            extension: "clock".into(),
                            id: invocation.call_id().as_str().into(),
                            title: "Wait completed".into(),
                            body: format!("Waited {elapsed} ms"),
                            status: extension_items::ExtensionItemStatus::Completed,
                            content: extension_items::ExtensionItemContent::Sleep {
                                duration_ms: ms as u32,
                            },
                        },
                    )
                    .map_err(|e| e.to_string())?;
                Ok(vec![ToolContent::Text(
                    json!({"elapsed_ms":elapsed}).to_string(),
                )])
            })();
            match result {
                Ok(output) => ToolExecutionOutcome::Returned(ToolOutput::success(output)),
                Err(error) => {
                    ToolExecutionOutcome::Returned(ToolOutput::error(vec![ToolContent::Text(
                        error,
                    )]))
                }
            }
        })
    }
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
