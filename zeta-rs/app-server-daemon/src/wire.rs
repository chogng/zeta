use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use zeta_uds::UnixStream;

use crate::ConnectionOptions;
use crate::WorkspaceTrustSource;

pub(crate) const CONNECTION_PRELUDE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(5);
const MAX_PRELUDE_BYTES: usize = 16 * 1024;
const PRELUDE_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectionPrelude {
    version: u32,
    pub(crate) workspace_root: Option<PathBuf>,
    pub(crate) workspace_trust_source: ConnectionWorkspaceTrustSource,
    pub(crate) product_services: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ConnectionWorkspaceTrustSource {
    HostConfiguration,
    UserConfig,
}

impl ConnectionPrelude {
    pub(crate) fn from_options(options: &ConnectionOptions) -> Self {
        Self {
            version: PRELUDE_VERSION,
            workspace_root: options.workspace_root().map(Path::to_path_buf),
            workspace_trust_source: match options.workspace_trust_source() {
                WorkspaceTrustSource::HostConfiguration => {
                    ConnectionWorkspaceTrustSource::HostConfiguration
                }
                WorkspaceTrustSource::UserConfig => ConnectionWorkspaceTrustSource::UserConfig,
            },
            product_services: options.product_services().map(Path::to_path_buf),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != PRELUDE_VERSION {
            return Err("unsupported local App Server connection prelude version".into());
        }
        Ok(())
    }

    pub(crate) fn trust_source(&self) -> WorkspaceTrustSource {
        match self.workspace_trust_source {
            ConnectionWorkspaceTrustSource::HostConfiguration => {
                WorkspaceTrustSource::HostConfiguration
            }
            ConnectionWorkspaceTrustSource::UserConfig => WorkspaceTrustSource::UserConfig,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ControlCommand {
    Status,
    Stop,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlPrelude {
    version: u32,
    kind: ControlPreludeKind,
    pub(crate) command: ControlCommand,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ControlPreludeKind {
    Control,
}

impl ControlPrelude {
    pub(crate) fn new(command: ControlCommand) -> Self {
        Self {
            version: PRELUDE_VERSION,
            kind: ControlPreludeKind::Control,
            command,
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != PRELUDE_VERSION {
            return Err("unsupported local App Server control prelude version".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum IncomingPrelude {
    Control(ControlPrelude),
    Connection(ConnectionPrelude),
}

impl IncomingPrelude {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Control(prelude) => prelude.validate(),
            Self::Connection(prelude) => prelude.validate(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ControlState {
    Running,
    Stopping,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlResponse {
    pub(crate) version: u32,
    pub(crate) state: ControlState,
    pub(crate) pid: u32,
    pub(crate) instance_id: String,
    pub(crate) daemon_version: String,
    pub(crate) schema_hash: String,
}

impl ControlResponse {
    pub(crate) fn new(
        state: ControlState,
        pid: u32,
        instance_id: String,
        schema_hash: String,
    ) -> Self {
        Self {
            version: PRELUDE_VERSION,
            state,
            pid,
            instance_id,
            daemon_version: env!("CARGO_PKG_VERSION").into(),
            schema_hash,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != PRELUDE_VERSION {
            return Err("unsupported local App Server control response version".into());
        }
        Ok(())
    }
}

pub(crate) fn read_prelude(reader: &mut BufReader<UnixStream>) -> Result<IncomingPrelude, String> {
    let mut line = String::new();
    let read = reader
        .by_ref()
        .take((MAX_PRELUDE_BYTES + 1) as u64)
        .read_line(&mut line)
        .map_err(io_error)?;
    if read == 0 || read > MAX_PRELUDE_BYTES || !line.ends_with('\n') {
        return Err("local App Server connection prelude is missing or too large".into());
    }
    let prelude: IncomingPrelude =
        serde_json::from_str(&line).map_err(|error| error.to_string())?;
    prelude.validate()?;
    Ok(prelude)
}

pub(crate) fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, value)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
