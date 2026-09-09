//! Versioned messages shared by the Zeta Windows sandbox service and its packaged client.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

pub const SANDBOX_SERVICE_NAME: &str = "ZetaSandboxService";
pub const SANDBOX_SERVICE_PIPE_NAME: &str = r"\\.\pipe\Zeta.Sandbox";
pub const SANDBOX_SERVICE_PROTOCOL_VERSION: u8 = 1;
pub const SANDBOX_WORKER_EXECUTABLE_NAME: &str = "zeta-windows-sandbox-worker.exe";
const MAX_FRAME_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowsSandboxProvisioningAccess {
    ReadOnly,
    DirectoryWrite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsSandboxProvisioningRequest {
    pub dir: PathBuf,
    pub program: PathBuf,
    pub access: WindowsSandboxProvisioningAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum WindowsSandboxProvisioningMessage {
    Provision(WindowsSandboxProvisioningRequest),
    Result(WindowsSandboxProvisioningResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum WindowsSandboxProvisioningResponse {
    Ok,
    Error { message: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsSandboxProvisioningFrame {
    pub version: u8,
    #[serde(flatten)]
    pub message: WindowsSandboxProvisioningMessage,
}

pub fn write_provisioning_frame(
    mut writer: impl Write,
    frame: &WindowsSandboxProvisioningFrame,
) -> Result<()> {
    let payload = serde_json::to_vec(frame).context("serialize Windows sandbox service frame")?;
    if payload.len() > MAX_FRAME_BYTES {
        bail!("Windows sandbox service frame exceeds {MAX_FRAME_BYTES} bytes");
    }
    let length = u32::try_from(payload.len()).context("size Windows sandbox service frame")?;
    writer
        .write_all(&length.to_le_bytes())
        .context("write Windows sandbox service frame length")?;
    writer
        .write_all(&payload)
        .context("write Windows sandbox service frame payload")
}

pub fn read_provisioning_frame(
    mut reader: impl Read,
) -> Result<Option<WindowsSandboxProvisioningFrame>> {
    let mut length = [0_u8; size_of::<u32>()];
    let mut offset = 0;
    while offset < length.len() {
        let read = reader
            .read(&mut length[offset..])
            .context("read Windows sandbox service frame length")?;
        if read == 0 {
            if offset == 0 {
                return Ok(None);
            }
            bail!("Windows sandbox service frame ended inside its length prefix");
        }
        offset += read;
    }
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_FRAME_BYTES {
        bail!("Windows sandbox service frame exceeds {MAX_FRAME_BYTES} bytes");
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .context("read Windows sandbox service frame payload")?;
    serde_json::from_slice(&payload)
        .context("parse Windows sandbox service frame")
        .map(Some)
}

#[cfg(test)]
#[path = "service_protocol_tests.rs"]
mod tests;
