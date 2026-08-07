use crate::DocumentCollaborationSnapshot;
use crate::DocumentCollaborationSubmitResult;
use crate::DocumentCollaborationUpdate;
use getrandom::getrandom;
use serde_json::Value;

pub(crate) const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_TRANSACTION_BYTES: usize = 1_048_576;
pub(crate) const MAX_ROOM_HISTORY: usize = 512;
pub(crate) const MAX_JAVASCRIPT_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Recoverable ordered history returned by a durable collaboration authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentCollaborationReplay {
    Updates(Vec<DocumentCollaborationUpdate>),
    Resync(DocumentCollaborationSnapshot),
}

pub(crate) fn validate_identifier(value: &str, name: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 128 {
        return Err(format!("{name} must contain between 1 and 128 characters"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(format!(
            "{name} may contain only letters, numbers, '-' and '_'"
        ));
    }
    Ok(())
}

pub(crate) fn validate_javascript_safe_integer(
    value: u64,
    name: &str,
    minimum: u64,
) -> Result<(), String> {
    if value < minimum || value > MAX_JAVASCRIPT_SAFE_INTEGER {
        return Err(format!(
            "{name} must be between {minimum} and {MAX_JAVASCRIPT_SAFE_INTEGER}"
        ));
    }
    Ok(())
}

pub(crate) fn validate_document(value: &str) -> Result<(), String> {
    validate_json_object(value, MAX_DOCUMENT_BYTES, "document")
}

pub(crate) fn validate_transaction(value: &str) -> Result<(), String> {
    validate_json_object(value, MAX_TRANSACTION_BYTES, "transaction")
}

pub(crate) fn snapshot(
    room_id: &str,
    version: u64,
    document: String,
) -> DocumentCollaborationSnapshot {
    DocumentCollaborationSnapshot {
        room_id: room_id.into(),
        version,
        document,
    }
}

pub(crate) fn replay(
    room_id: &str,
    version: u64,
    document: String,
    updates: Vec<DocumentCollaborationUpdate>,
    base_version: u64,
) -> DocumentCollaborationReplay {
    let history_start = updates
        .first()
        .map(|update| update.base_version)
        .unwrap_or(version);
    if base_version < history_start {
        return DocumentCollaborationReplay::Resync(snapshot(room_id, version, document));
    }
    DocumentCollaborationReplay::Updates(
        updates
            .into_iter()
            .filter(|update| update.version > base_version)
            .collect(),
    )
}

pub(crate) fn replay_submit_result(
    replay: DocumentCollaborationReplay,
) -> DocumentCollaborationSubmitResult {
    match replay {
        DocumentCollaborationReplay::Updates(updates) => {
            DocumentCollaborationSubmitResult::Conflict { updates }
        }
        DocumentCollaborationReplay::Resync(snapshot) => {
            DocumentCollaborationSubmitResult::Resync { snapshot }
        }
    }
}

pub(crate) fn random_room_id() -> Result<String, String> {
    let mut random = [0_u8; 16];
    getrandom(&mut random)
        .map_err(|error| format!("Could not create a collaboration room ID: {error}"))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut room_id = String::from("gama-");
    for byte in random {
        room_id.push(HEX[(byte >> 4) as usize] as char);
        room_id.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(room_id)
}

fn validate_json_object(value: &str, maximum_bytes: usize, name: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > maximum_bytes {
        return Err(format!(
            "{name} must contain between 1 and {maximum_bytes} bytes"
        ));
    }
    let json: Value =
        serde_json::from_str(value).map_err(|_| format!("{name} must contain valid JSON"))?;
    if !json.is_object() {
        return Err(format!("{name} must contain a JSON object"));
    }
    Ok(())
}
