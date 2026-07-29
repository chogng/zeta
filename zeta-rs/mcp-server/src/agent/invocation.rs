use super::{ReplyAgentRequest, StartAgentRequest};
use sha2::{Digest, Sha256};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InvocationFingerprint([u8; 32]);

impl InvocationFingerprint {
    pub(crate) fn encode(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

pub(crate) fn start_fingerprint(request: &StartAgentRequest) -> InvocationFingerprint {
    fingerprint([
        "start",
        &request.prompt,
        &duration_fingerprint(request.timeout),
    ])
}

pub(super) fn reply_fingerprint(request: &ReplyAgentRequest) -> InvocationFingerprint {
    fingerprint([
        "reply",
        &request.thread_id,
        &request.prompt,
        &duration_fingerprint(request.timeout),
    ])
}

fn duration_fingerprint(duration: Option<Duration>) -> String {
    duration
        .map(|value| value.as_millis().to_string())
        .unwrap_or_else(|| "default".into())
}

fn fingerprint<const N: usize>(parts: [&str; N]) -> InvocationFingerprint {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part.len().to_le_bytes());
        digest.update(part.as_bytes());
    }
    InvocationFingerprint(digest.finalize().into())
}
