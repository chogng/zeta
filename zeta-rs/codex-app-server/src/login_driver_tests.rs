use super::CodexAppServerLoginDriver;
use super::CodexAppServerOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use zeta_login::BeginLogin;
use zeta_login::CancelLoginOutcome;
use zeta_login::LoginErrorKind;
use zeta_login::LoginMethod;
use zeta_login::LoginService;
use zeta_login::LogoutOutcome;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
#[cfg(unix)]
fn browser_login_round_trips_through_the_upstream_process_without_credentials() {
    let (_root, program) = fake_codex_program();
    let driver = CodexAppServerLoginDriver::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let service = Arc::new(LoginService::deferred(driver.clone()));
    driver.install_login_service(&service).unwrap();

    assert_eq!(service.refresh().unwrap().account, None);
    let started = service.begin(LoginMethod::OpenAiChatGptBrowser).unwrap();
    assert!(matches!(
        started,
        BeginLogin::Browser {
            authorization_url,
            ..
        } if authorization_url == "https://auth.example.test/start"
    ));
    wait_until(|| service.read().unwrap().account.is_some());
    let account = service.read().unwrap().account.unwrap();
    assert_eq!(account.account.provider, "openai-chatgpt");
    assert_eq!(account.email.as_deref(), Some("person@example.test"));
    assert_eq!(account.plan.as_deref(), Some("pro"));

    assert_eq!(service.logout().unwrap(), LogoutOutcome::LoggedOut);
    wait_until(|| service.read().unwrap().account.is_none());
}

#[test]
#[cfg(unix)]
fn device_code_login_preserves_local_identity_and_exact_cancellation() {
    let (_root, program) = fake_codex_program();
    let driver = CodexAppServerLoginDriver::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let service = Arc::new(LoginService::deferred(driver.clone()));
    driver.install_login_service(&service).unwrap();

    let started = service.begin(LoginMethod::OpenAiChatGptDeviceCode).unwrap();
    let login_id = match started {
        BeginLogin::DeviceCode {
            login_id,
            verification_url,
            user_code,
        } => {
            assert_eq!(verification_url, "https://auth.example.test/device");
            assert_eq!(user_code, "ZETA-CODE");
            login_id
        }
        other => panic!("expected device-code login, got {other:?}"),
    };
    let conflict = service
        .begin(LoginMethod::OpenAiChatGptBrowser)
        .unwrap_err();
    assert_eq!(conflict.kind(), LoginErrorKind::Conflict);
    assert_eq!(
        service.cancel(&login_id).unwrap(),
        CancelLoginOutcome::Cancelled
    );
    assert_eq!(
        service.cancel(&login_id).unwrap(),
        CancelLoginOutcome::NotFound
    );
}

#[test]
fn unavailable_binary_is_reported_only_when_the_deferred_driver_is_used() {
    let driver = CodexAppServerLoginDriver::new(
        CodexAppServerOptions::new("/zeta/does/not/exist/codex")
            .with_request_timeout(Duration::from_millis(50)),
    );
    let service = Arc::new(LoginService::deferred(driver.clone()));
    driver.install_login_service(&service).unwrap();

    let error = service.refresh().unwrap_err();
    assert_eq!(error.kind(), LoginErrorKind::Unavailable);
}

#[test]
#[cfg(unix)]
fn unknown_server_request_is_answered_without_blocking_account_reads() {
    let (_root, program) = fake_codex_with_server_request();
    let driver = CodexAppServerLoginDriver::new(
        CodexAppServerOptions::new(program).with_request_timeout(Duration::from_secs(10)),
    );
    let service = Arc::new(LoginService::deferred(driver.clone()));
    driver.install_login_service(&service).unwrap();

    assert_eq!(service.refresh().unwrap().account, None);
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(Instant::now() < deadline, "condition did not become true");
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn fake_codex_program() -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    std::fs::write(
        &program,
        r#"#!/bin/sh
logged_in=0
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}"
      ;;
    *'"method":"account/read"'*)
      if [ "$logged_in" = 1 ]; then
        printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"account\":{\"type\":\"chatgpt\",\"email\":\"person@example.test\",\"planType\":\"pro\"},\"requiresOpenaiAuth\":true}}"
      else
        printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"account\":null,\"requiresOpenaiAuth\":true}}"
      fi
      ;;
    *'"method":"account/login/start"'*'"chatgptDeviceCode"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"type\":\"chatgptDeviceCode\",\"loginId\":\"upstream-device\",\"verificationUrl\":\"https://auth.example.test/device\",\"userCode\":\"ZETA-CODE\"}}"
      ;;
    *'"method":"account/login/start"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"type\":\"chatgpt\",\"loginId\":\"upstream-browser\",\"authUrl\":\"https://auth.example.test/start\"}}"
      logged_in=1
      printf '%s\n' '{"jsonrpc":"2.0","method":"account/login/completed","params":{"loginId":"upstream-browser","success":true,"error":null}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"account/updated","params":{"authMode":"chatgpt","planType":"pro"}}'
      ;;
    *'"method":"account/login/cancel"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"status\":\"canceled\"}}"
      ;;
    *'"method":"account/logout"'*)
      logged_in=0
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}"
      printf '%s\n' '{"jsonrpc":"2.0","method":"account/updated","params":{"authMode":null,"planType":null}}'
      ;;
  esac
done
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}

#[cfg(unix)]
fn fake_codex_with_server_request() -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let program = root.path().join("codex");
    std::fs::write(
        &program,
        r#"#!/bin/sh
pending_read_id=
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"userAgent\":\"codex-test/1.0\",\"codexHome\":\"/tmp/codex-test\",\"platformFamily\":\"unix\",\"platformOs\":\"test\"}}"
      ;;
    *'"method":"account/read"'*)
      pending_read_id=$id
      printf '%s\n' '{"jsonrpc":"2.0","id":"server-1","method":"unknown/request","params":{}}'
      ;;
    *'"id":"server-1"'*|*'"error"'*)
      case "$line" in
        *'"id":"server-1"'*) ;;
        *) continue ;;
      esac
      case "$line" in
        *'"error"'*) ;;
        *) continue ;;
      esac
      printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":$pending_read_id,\"result\":{\"account\":null,\"requiresOpenaiAuth\":true}}"
      pending_read_id=
      ;;
  esac
done
"#,
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&program).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&program, permissions).unwrap();
    (root, program)
}
