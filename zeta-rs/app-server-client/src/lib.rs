//! Reusable typed app-server client boundary and contract-test entry point.

mod in_process;
mod notification;

use serde_json::Value;
use serde_json::json;
use std::fmt;
use zeta_app_server_protocol::v1::config::ConfigReadResult;
use zeta_app_server_protocol::v1::config::ConfigUpdateParams;
use zeta_app_server_protocol::v1::initialize::InitializeParams;
use zeta_app_server_protocol::v1::initialize::InitializeResult;
use zeta_app_server_protocol::v1::resources::ResourceMetadataParams;
use zeta_app_server_protocol::v1::resources::ResourceMetadataResult;
use zeta_app_server_protocol::v1::resources::ResourceReadParams;
use zeta_app_server_protocol::v1::resources::ResourceReadResult;
use zeta_app_server_protocol::v1::resources::ResourceReleaseParams;
use zeta_app_server_protocol::v1::thread::ThreadListResult;
use zeta_app_server_protocol::v1::thread::ThreadReadParams;
use zeta_app_server_protocol::v1::thread::ThreadReadResult;
use zeta_app_server_protocol::v1::thread::ThreadResumeParams;
use zeta_app_server_protocol::v1::thread::ThreadStartParams;
use zeta_app_server_protocol::v1::thread::ThreadStartResult;
use zeta_app_server_protocol::v1::thread::ThreadUnsubscribeParams;
use zeta_app_server_protocol::v1::turn::TurnInterruptParams;
use zeta_app_server_protocol::v1::turn::TurnStartParams;
use zeta_app_server_protocol::v1::turn::TurnStartResult;

pub use in_process::InProcessClientOptions;
pub use in_process::InProcessTransport;
pub use in_process::start_in_process_client;
pub use notification::ServerNotification;

/// Exchanges one complete JSON-RPC request with a connected app-server transport.
///
/// Implementations must preserve request/response pairing for a single connection and must not
/// return notifications as if they were responses to this client call. Implementations buffer
/// causally emitted notifications until `drain_notifications` is called.
pub trait JsonRpcTransport {
    fn round_trip(&mut self, request: &str) -> Result<String, ClientError>;

    fn drain_notifications(&mut self) -> Result<Vec<String>, ClientError> {
        Ok(Vec::new())
    }
}

pub struct AppServerClient<T> {
    transport: T,
    next_request_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    Transport(String),
    Protocol(String),
    Server { code: i64, message: String },
}

impl<T: JsonRpcTransport> AppServerClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_request_id: 1,
        }
    }

    pub fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> Result<InitializeResult, ClientError> {
        self.call("initialize", params)
    }

    pub fn start_thread(
        &mut self,
        params: ThreadStartParams,
    ) -> Result<ThreadStartResult, ClientError> {
        self.call("thread/start", params)
    }

    pub fn read_thread(
        &mut self,
        params: ThreadReadParams,
    ) -> Result<ThreadReadResult, ClientError> {
        self.call("thread/read", params)
    }

    pub fn resume_thread(
        &mut self,
        params: ThreadResumeParams,
    ) -> Result<ThreadReadResult, ClientError> {
        self.call("thread/resume", params)
    }

    pub fn list_threads(&mut self) -> Result<ThreadListResult, ClientError> {
        self.call("thread/list", json!({}))
    }

    pub fn unsubscribe_thread(
        &mut self,
        params: ThreadUnsubscribeParams,
    ) -> Result<(), ClientError> {
        self.call("thread/unsubscribe", params)
    }

    pub fn read_config(&mut self) -> Result<ConfigReadResult, ClientError> {
        self.call("config/read", json!({}))
    }

    pub fn update_config(
        &mut self,
        params: ConfigUpdateParams,
    ) -> Result<ConfigReadResult, ClientError> {
        self.call("config/update", params)
    }

    pub fn start_turn(&mut self, params: TurnStartParams) -> Result<TurnStartResult, ClientError> {
        self.call("turn/start", params)
    }

    pub fn interrupt_turn(&mut self, params: TurnInterruptParams) -> Result<(), ClientError> {
        self.call("turn/interrupt", params)
    }

    pub fn resource_metadata(
        &mut self,
        params: ResourceMetadataParams,
    ) -> Result<ResourceMetadataResult, ClientError> {
        self.call("resource/metadata", params)
    }

    pub fn read_resource(
        &mut self,
        params: ResourceReadParams,
    ) -> Result<ResourceReadResult, ClientError> {
        self.call("resource/read", params)
    }

    pub fn release_resource(&mut self, params: ResourceReleaseParams) -> Result<(), ClientError> {
        self.call("resource/release", params)
    }

    pub fn drain_notifications(&mut self) -> Result<Vec<ServerNotification>, ClientError> {
        self.transport
            .drain_notifications()?
            .into_iter()
            .map(|raw| notification::decode(&raw))
            .collect()
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    fn call<P: serde::Serialize, R: for<'a> serde::Deserialize<'a>>(
        &mut self,
        method: &str,
        params: P,
    ) -> Result<R, ClientError> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let params = serde_json::to_value(params)
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        let request =
            json!({ "jsonrpc": "2.0", "id": request_id, "method": method, "params": params });
        let raw_response = self.transport.round_trip(&request.to_string())?;
        let response: Value = serde_json::from_str(&raw_response)
            .map_err(|error| ClientError::Protocol(error.to_string()))?;
        if response.get("id") != Some(&json!(request_id)) {
            return Err(ClientError::Protocol(
                "response id did not match request".into(),
            ));
        }
        if let Some(error) = response.get("error") {
            return Err(ClientError::Server {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(-32000),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .into(),
            });
        }
        serde_json::from_value(
            response
                .get("result")
                .cloned()
                .ok_or_else(|| ClientError::Protocol("response has no result".into()))?,
        )
        .map_err(|error| ClientError::Protocol(error.to_string()))
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => write!(formatter, "transport error: {message}"),
            Self::Protocol(message) => write!(formatter, "protocol error: {message}"),
            Self::Server { code, message } => {
                write!(formatter, "server error {code}: {message}")
            }
        }
    }
}

impl std::error::Error for ClientError {}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
