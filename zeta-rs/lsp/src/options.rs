use std::sync::Arc;
use std::time::Duration;

use lsp_types::{
    ClientCapabilities, ClientInfo, GeneralClientCapabilities, PositionEncodingKind,
    PublishDiagnosticsClientCapabilities, TextDocumentClientCapabilities,
    TextDocumentSyncClientCapabilities, Uri, WorkspaceClientCapabilities, WorkspaceFolder,
};
use serde_json::Value;

use crate::{LanguageServerHost, NoopLanguageServerHost};

/// Deadlines applied independently to initialization, normal requests, and shutdown.
#[derive(Clone, Copy, Debug)]
pub struct LanguageServerTimeouts {
    pub initialize: Duration,
    pub request: Duration,
    pub shutdown: Duration,
}

impl Default for LanguageServerTimeouts {
    fn default() -> Self {
        Self {
            initialize: Duration::from_secs(10),
            request: Duration::from_secs(30),
            shutdown: Duration::from_secs(5),
        }
    }
}

/// Client identity, workspace context, capabilities, callbacks, and deadlines for one server.
#[derive(Clone)]
pub struct LanguageServerOptions {
    pub(crate) client_info: ClientInfo,
    pub(crate) root_uri: Option<Uri>,
    pub(crate) workspace_folders: Option<Vec<WorkspaceFolder>>,
    pub(crate) initialization_options: Option<Value>,
    pub(crate) capabilities: ClientCapabilities,
    pub(crate) locale: Option<String>,
    pub(crate) host: Arc<dyn LanguageServerHost>,
    pub(crate) timeouts: LanguageServerTimeouts,
}

impl std::fmt::Debug for LanguageServerOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LanguageServerOptions")
            .field("client_info", &self.client_info)
            .field("root_uri", &self.root_uri)
            .field("workspace_folders", &self.workspace_folders)
            .field("initialization_options", &self.initialization_options)
            .field("capabilities", &self.capabilities)
            .field("locale", &self.locale)
            .field("host", &"<dyn LanguageServerHost>")
            .field("timeouts", &self.timeouts)
            .finish()
    }
}

impl LanguageServerOptions {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            client_info: ClientInfo {
                name: name.into(),
                version: Some(version.into()),
            },
            root_uri: None,
            workspace_folders: None,
            initialization_options: None,
            capabilities: default_client_capabilities(),
            locale: None,
            host: Arc::new(NoopLanguageServerHost),
            timeouts: LanguageServerTimeouts::default(),
        }
    }

    pub fn with_root_uri(mut self, root_uri: Uri) -> Self {
        self.root_uri = Some(root_uri);
        self
    }

    pub fn with_workspace_folders(mut self, workspace_folders: Vec<WorkspaceFolder>) -> Self {
        self.workspace_folders = Some(workspace_folders);
        self
    }

    pub fn with_initialization_options(mut self, initialization_options: Value) -> Self {
        self.initialization_options = Some(initialization_options);
        self
    }

    pub fn with_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn with_host(mut self, host: Arc<dyn LanguageServerHost>) -> Self {
        self.host = host;
        self
    }

    pub fn with_timeouts(mut self, timeouts: LanguageServerTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }
}

fn default_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            configuration: Some(true),
            workspace_folders: Some(true),
            ..Default::default()
        }),
        text_document: Some(TextDocumentClientCapabilities {
            synchronization: Some(TextDocumentSyncClientCapabilities {
                dynamic_registration: Some(false),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                did_save: Some(true),
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities::default()),
            ..Default::default()
        }),
        general: Some(GeneralClientCapabilities {
            position_encodings: Some(vec![
                PositionEncodingKind::UTF8,
                PositionEncodingKind::UTF16,
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}
