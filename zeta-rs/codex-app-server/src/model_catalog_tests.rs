use super::CodexAppServerOptions;
use super::CodexAppServerRuntime;
use super::CodexModelCatalog;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
#[cfg(unix)]
fn chatgpt_catalog_uses_model_ids_and_bounded_pagination() {
    let (root, program) = fake_catalog_program(CatalogAccount::ChatGpt);
    let runtime = CodexAppServerRuntime::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );

    let models = CodexModelCatalog::new(runtime).list().unwrap();

    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "gpt-codex-default");
    assert_eq!(models[0].display_name, "Codex Default");
    assert!(models[0].is_default);
    assert_eq!(models[1].id, "gpt-codex-fast");
    assert_eq!(models[1].display_name, "Codex Fast");
    assert!(!models[1].is_default);
    let requests = std::fs::read_to_string(root.path().join("requests.log")).unwrap();
    assert_eq!(requests.matches("\"method\":\"model/list\"").count(), 2);
    assert!(requests.contains("\"includeHidden\":false"));
    assert!(requests.contains("\"limit\":100"));
    assert!(requests.contains("\"cursor\":\"next-page\""));
}

#[test]
#[cfg(unix)]
fn catalog_is_empty_without_a_chatgpt_account() {
    let (root, program) = fake_catalog_program(CatalogAccount::None);
    let runtime = CodexAppServerRuntime::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );

    assert!(CodexModelCatalog::new(runtime).list().unwrap().is_empty());

    let requests = std::fs::read_to_string(root.path().join("requests.log")).unwrap();
    assert_eq!(requests.matches("\"method\":\"account/read\"").count(), 1);
    assert!(!requests.contains("\"method\":\"model/list\""));
}

#[derive(Clone, Copy)]
enum CatalogAccount {
    ChatGpt,
    None,
}

#[cfg(unix)]
fn fake_catalog_program(account: CatalogAccount) -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    let request_log = root.path().join("requests.log");
    let account_result = match account {
        CatalogAccount::ChatGpt => {
            r#"{\"account\":{\"type\":\"chatgpt\",\"email\":\"hidden@example.invalid\"}}"#
        }
        CatalogAccount::None => r#"{\"account\":null}"#,
    };
    let script = format!(
        r#"#!/bin/sh
while IFS= read -r line; do
  printf '%s\n' "$line" >> '{request_log}'
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}}}"
      ;;
    *'"method":"account/read"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{account_result}}}"
      ;;
    *'"method":"model/list"'*'"cursor":"next-page"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"data\":[{{\"model\":\"gpt-codex-fast\",\"displayName\":\"Codex Fast\",\"isDefault\":false}}],\"nextCursor\":null}}}}"
      ;;
    *'"method":"model/list"'*)
      printf '%s\n' "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"data\":[{{\"model\":\"gpt-codex-default\",\"displayName\":\"Codex Default\",\"isDefault\":true}}],\"nextCursor\":\"next-page\"}}}}"
      ;;
  esac
done
"#,
        request_log = request_log.display(),
    );
    std::fs::write(&program, script).unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}
