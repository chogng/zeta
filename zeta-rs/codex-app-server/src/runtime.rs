use crate::CodexAppServerOptions;
use crate::process::CodexAppServerProcess;
use crate::process::ProcessError;
use crate::process::ProcessErrorKind;
use crate::process::UpstreamEvent;
use serde_json::Value;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;

pub(crate) enum EventHandling {
    Ignored,
    Handled,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct UpstreamConnectionId(u64);

pub(crate) trait UpstreamEventHandler: Send + Sync {
    fn handle_event(
        &self,
        connection_id: UpstreamConnectionId,
        event: &UpstreamEvent,
    ) -> EventHandling;
}

struct ActiveProcess {
    connection_id: UpstreamConnectionId,
    process: Arc<CodexAppServerProcess>,
}

/// Shared, lazily started connection to one upstream Codex App Server.
///
/// Account and Turn adapters install narrow event handlers on this runtime so
/// one process owns request IDs, server requests, notifications, and shutdown.
/// No credential material crosses this boundary.
pub struct CodexAppServerRuntime {
    options: CodexAppServerOptions,
    self_weak: Weak<Self>,
    process: Mutex<Option<ActiveProcess>>,
    next_connection_id: AtomicU64,
    handlers: Mutex<Vec<Weak<dyn UpstreamEventHandler>>>,
}

impl CodexAppServerRuntime {
    pub fn new(options: CodexAppServerOptions) -> Arc<Self> {
        Arc::new_cyclic(|self_weak| Self {
            options,
            self_weak: self_weak.clone(),
            process: Mutex::new(None),
            next_connection_id: AtomicU64::new(1),
            handlers: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn install_handler(&self, handler: &Arc<dyn UpstreamEventHandler>) {
        self.handlers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(Arc::downgrade(handler));
    }

    pub(crate) fn request(&self, method: &str, params: Value) -> Result<Value, ProcessError> {
        let (connection_id, process) = self.process()?;
        match process.request(method, params) {
            Ok(value) => Ok(value),
            Err(error) => {
                if error.kind == ProcessErrorKind::Unavailable {
                    self.clear_process(connection_id, &process);
                }
                Err(error)
            }
        }
    }

    pub(crate) fn request_without_params(&self, method: &str) -> Result<Value, ProcessError> {
        let (connection_id, process) = self.process()?;
        match process.request_without_params(method) {
            Ok(value) => Ok(value),
            Err(error) => {
                if error.kind == ProcessErrorKind::Unavailable {
                    self.clear_process(connection_id, &process);
                }
                Err(error)
            }
        }
    }

    pub(crate) fn respond(
        &self,
        connection_id: UpstreamConnectionId,
        id: Value,
        result: Value,
    ) -> Result<(), ProcessError> {
        let process = self.process_for_response(connection_id)?;
        let response = process.respond(id, result);
        if response.is_err() {
            self.clear_process(connection_id, &process);
        }
        response
    }

    pub(crate) fn respond_error(
        &self,
        connection_id: UpstreamConnectionId,
        id: Value,
        code: i64,
        message: &'static str,
    ) -> Result<(), ProcessError> {
        let process = self.process_for_response(connection_id)?;
        let response = process.respond_error(id, code, message);
        if response.is_err() {
            self.clear_process(connection_id, &process);
        }
        response
    }

    fn process(&self) -> Result<(UpstreamConnectionId, Arc<CodexAppServerProcess>), ProcessError> {
        let mut current = self.process.lock().map_err(|_| runtime_unavailable())?;
        if let Some(active) = current.as_ref() {
            return Ok((active.connection_id, Arc::clone(&active.process)));
        }
        let (process, events) =
            CodexAppServerProcess::start(&self.options.program, self.options.request_timeout)?;
        let connection_id =
            UpstreamConnectionId(self.next_connection_id.fetch_add(1, Ordering::Relaxed));
        let weak = self.self_weak.clone();
        thread::Builder::new()
            .name("zeta-codex-app-server-events".into())
            .spawn(move || {
                while let Ok(event) = events.recv() {
                    let Some(runtime) = weak.upgrade() else {
                        return;
                    };
                    runtime.dispatch(connection_id, event);
                }
                if let Some(runtime) = weak.upgrade() {
                    runtime.dispatch(connection_id, UpstreamEvent::ConnectionClosed);
                    runtime.clear_connection(connection_id);
                }
            })
            .map_err(|_| runtime_unavailable())?;
        *current = Some(ActiveProcess {
            connection_id,
            process: Arc::clone(&process),
        });
        Ok((connection_id, process))
    }

    fn process_for_response(
        &self,
        connection_id: UpstreamConnectionId,
    ) -> Result<Arc<CodexAppServerProcess>, ProcessError> {
        self.process
            .lock()
            .map_err(|_| runtime_unavailable())?
            .as_ref()
            .filter(|active| active.connection_id == connection_id)
            .map(|active| Arc::clone(&active.process))
            .ok_or_else(|| {
                ProcessError::unavailable(
                    "Codex server request belongs to a closed upstream connection",
                )
            })
    }

    fn dispatch(&self, connection_id: UpstreamConnectionId, event: UpstreamEvent) {
        let handlers = {
            let Ok(mut handlers) = self.handlers.lock() else {
                return;
            };
            let active = handlers
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>();
            handlers.retain(|handler| handler.strong_count() > 0);
            active
        };
        let mut handled = false;
        for handler in handlers {
            if matches!(
                handler.handle_event(connection_id, &event),
                EventHandling::Handled
            ) {
                handled = true;
                if matches!(&event, UpstreamEvent::Request { .. }) {
                    break;
                }
            }
        }
        if !handled && let UpstreamEvent::Request { id, .. } = event {
            let _ = self.respond_error(
                connection_id,
                id,
                -32601,
                "Codex server request is not supported by this Zeta runtime",
            );
        }
    }

    fn clear_process(
        &self,
        connection_id: UpstreamConnectionId,
        failed: &Arc<CodexAppServerProcess>,
    ) {
        if let Ok(mut current) = self.process.lock()
            && current.as_ref().is_some_and(|current| {
                current.connection_id == connection_id && Arc::ptr_eq(&current.process, failed)
            })
        {
            *current = None;
        }
    }

    fn clear_connection(&self, connection_id: UpstreamConnectionId) {
        if let Ok(mut current) = self.process.lock()
            && current
                .as_ref()
                .is_some_and(|current| current.connection_id == connection_id)
        {
            *current = None;
        }
    }
}

fn runtime_unavailable() -> ProcessError {
    ProcessError {
        kind: ProcessErrorKind::Unavailable,
        message: "Codex App Server runtime state was unavailable",
    }
}
