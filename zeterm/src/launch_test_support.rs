#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;

#[cfg(unix)]
use serde_json::json;

#[cfg(unix)]
pub(crate) fn initialize_response(server_schema_hash: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "serverInfo": { "name": "fake-remote", "version": "1" },
            "schemaHash": server_schema_hash,
            "capabilities": {
                "agentInteractions": false,
                "documentCollaboration": false,
                "sessions": false,
                "threads": false,
                "turns": false,
                "resources": false,
                "attachments": false,
                "fileSystem": false,
                "git": false,
                "workspaceSearch": false,
                "codeIndex": false,
                "cloudCodeIndex": false,
                "terminal": false,
                "debugAdapter": false,
                "typst": false,
                "updateReplay": false,
                "extensions": false,
                "extensionHost": false,
                "connectors": false,
                "plugins": false,
                "marketplace": false,
                "mcp": false,
                "mcpOAuth": false
            },
            "slashCommands": []
        }
    })
    .to_string()
}

#[cfg(unix)]
pub(crate) fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}
