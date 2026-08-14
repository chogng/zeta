use super::StdioAppServerCommand;
use super::request_id;
use super::response_id;
use std::path::Path;

#[test]
fn stdio_command_preserves_process_arguments_and_environment() {
    let command = StdioAppServerCommand::new("zeta-remote-server")
        .with_argument("app-server")
        .with_environment_variable("ZETA_WORKSPACE_ROOT", "/srv/zeta");

    assert_eq!(command.executable(), Path::new("zeta-remote-server"));
    assert_eq!(command.arguments_as_strings(), vec!["app-server"]);
    assert_eq!(command.environment.len(), 1);
}

#[test]
fn stdio_driver_accepts_only_positive_numeric_request_and_response_ids() {
    assert_eq!(
        request_id(r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#).unwrap(),
        7
    );
    assert_eq!(
        response_id(r#"{"jsonrpc":"2.0","id":7,"result":{}}"#).unwrap(),
        7
    );
    assert!(
        request_id(r#"{"jsonrpc":"2.0","id":"seven","method":"initialize","params":{}}"#).is_err()
    );
    assert!(response_id(r#"{"jsonrpc":"2.0","id":0,"result":{}}"#).is_err());
}
