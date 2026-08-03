use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_protocol::CommandId;

/// Creates the stable identity for one newly initiated logical command.
pub(crate) fn new_command_id(prefix: &str) -> CommandId {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    CommandId::new(format!("{prefix}-{}-{timestamp}", std::process::id()))
        .expect("generated command ID is non-empty")
}

#[cfg(test)]
#[path = "command_id_tests.rs"]
mod tests;
