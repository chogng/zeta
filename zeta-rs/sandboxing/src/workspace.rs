/// Workspace metadata names that writable sandbox backends protect when the paths are present.
///
/// Platform backends must apply these protections beneath every workspace root so an Agent cannot
/// persist execution hooks or weaken its own durable instructions while performing ordinary edits.
pub const PROTECTED_WORKSPACE_METADATA_NAMES: &[&str] = &[".git", ".agents", ".codex", ".zeta"];
