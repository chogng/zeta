use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use zeta_async_utils::CancellationToken;
use zeta_mcp::{
    McpCallError, McpRuntime, McpRuntimeOptions, McpServerDefinition, McpSessionFactory,
    McpToolBinding, RmcpSessionFactory,
};
use zeta_protocol::{ToolDefinition, ToolName};
use zeta_tools::ToolOutput;

const MCP_COMMAND_QUEUE_CAPACITY: usize = 64;

enum RuntimeCommand {
    Call {
        binding: McpToolBinding,
        arguments: serde_json::Value,
        cancellation: CancellationToken,
        response: mpsc::Sender<Result<ToolOutput, McpCallError>>,
    },
    Shutdown,
}

struct RuntimeStartup {
    definitions: Vec<ToolDefinition>,
    bindings: BTreeMap<ToolName, McpToolBinding>,
}

/// Synchronous App Server handle for one continuously driven async MCP runtime.
///
/// The worker thread owns the Tokio runtime and all live MCP sessions. Callers may block on the
/// response channel, while cancellation remains independently wakeable on the worker.
pub(crate) struct McpRuntimeOwner {
    commands: Option<tokio::sync::mpsc::Sender<RuntimeCommand>>,
    definitions: Vec<ToolDefinition>,
    bindings: BTreeMap<ToolName, McpToolBinding>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl McpRuntimeOwner {
    pub(crate) fn start(
        definitions: Vec<McpServerDefinition>,
        options: McpRuntimeOptions,
    ) -> Result<Self, McpRuntimeOwnerError> {
        Self::start_with_factory(definitions, options, Arc::new(RmcpSessionFactory))
    }

    pub(crate) fn start_with_factory(
        definitions: Vec<McpServerDefinition>,
        options: McpRuntimeOptions,
        factory: Arc<dyn McpSessionFactory>,
    ) -> Result<Self, McpRuntimeOwnerError> {
        let (commands, receiver) = tokio::sync::mpsc::channel(MCP_COMMAND_QUEUE_CAPACITY);
        let (startup_sender, startup_receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("zeta-mcp-runtime".into())
            .spawn(move || run_worker(definitions, options, factory, receiver, startup_sender))
            .map_err(|error| McpRuntimeOwnerError(error.to_string()))?;
        let startup = match startup_receiver.recv() {
            Ok(Ok(startup)) => startup,
            Ok(Err(error)) => {
                let _ = worker.join();
                return Err(McpRuntimeOwnerError(error));
            }
            Err(_) => {
                let _ = worker.join();
                return Err(McpRuntimeOwnerError(
                    "MCP runtime worker stopped during startup".into(),
                ));
            }
        };
        Ok(Self {
            commands: Some(commands),
            definitions: startup.definitions,
            bindings: startup.bindings,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub(crate) fn definitions(&self) -> &[ToolDefinition] {
        &self.definitions
    }

    pub(crate) fn resolve(&self, name: &ToolName) -> Option<&McpToolBinding> {
        self.bindings.get(name)
    }

    pub(crate) fn call(
        &self,
        binding: McpToolBinding,
        arguments: serde_json::Value,
        cancellation: CancellationToken,
    ) -> Result<ToolOutput, McpCallError> {
        let (response, receiver) = mpsc::channel();
        self.commands
            .as_ref()
            .ok_or_else(|| McpCallError::NotStarted("MCP runtime is shutting down".into()))?
            .try_send(RuntimeCommand::Call {
                binding,
                arguments,
                cancellation,
                response,
            })
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => {
                    McpCallError::NotStarted("MCP runtime command queue is full".into())
                }
                tokio::sync::mpsc::error::TrySendError::Closed(_) => {
                    McpCallError::NotStarted("MCP runtime worker is unavailable".into())
                }
            })?;
        receiver.recv().map_err(|_| {
            McpCallError::OutcomeUncertain(
                "MCP runtime worker stopped before reporting an outcome".into(),
            )
        })?
    }
}

impl Drop for McpRuntimeOwner {
    fn drop(&mut self) {
        if let Some(commands) = self.commands.take() {
            let _ = commands.try_send(RuntimeCommand::Shutdown);
            drop(commands);
        }
        if let Ok(worker) = self.worker.get_mut()
            && let Some(worker) = worker.take()
        {
            let _ = worker.join();
        }
    }
}

fn run_worker(
    definitions: Vec<McpServerDefinition>,
    options: McpRuntimeOptions,
    factory: Arc<dyn McpSessionFactory>,
    mut commands: tokio::sync::mpsc::Receiver<RuntimeCommand>,
    startup: mpsc::Sender<Result<RuntimeStartup, String>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = startup.send(Err(error.to_string()));
            return;
        }
    };
    runtime.block_on(async move {
        let mcp = match McpRuntime::start_with_factory(definitions, factory, options).await {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = startup.send(Err(error.to_string()));
                return;
            }
        };
        let definitions = match mcp.catalog().model_definitions() {
            Ok(definitions) => definitions,
            Err(error) => {
                let _ = mcp.shutdown().await;
                let _ = startup.send(Err(error.to_string()));
                return;
            }
        };
        let bindings = mcp
            .catalog()
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.binding().exposed_name().clone(),
                    tool.binding().clone(),
                )
            })
            .collect();
        if startup
            .send(Ok(RuntimeStartup {
                definitions,
                bindings,
            }))
            .is_err()
        {
            let _ = mcp.shutdown().await;
            return;
        }

        let mcp = Arc::new(mcp);
        let mut calls = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                command = commands.recv() => {
                    match command {
                        Some(RuntimeCommand::Call {
                            binding,
                            arguments,
                            cancellation,
                            response,
                        }) => {
                            let mcp = Arc::clone(&mcp);
                            calls.spawn(async move {
                                let result = mcp
                                    .call_tool(&binding, arguments, &cancellation)
                                    .await;
                                let _ = response.send(result);
                            });
                        }
                        Some(RuntimeCommand::Shutdown) | None => break,
                    }
                }
                Some(_) = calls.join_next(), if !calls.is_empty() => {}
            }
        }
        calls.abort_all();
        while calls.join_next().await.is_some() {}
        if let Ok(mcp) = Arc::try_unwrap(mcp) {
            let _ = mcp.shutdown().await;
        }
    });
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct McpRuntimeOwnerError(String);

impl std::fmt::Display for McpRuntimeOwnerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for McpRuntimeOwnerError {}

#[cfg(test)]
#[path = "mcp_runtime_tests.rs"]
mod tests;
