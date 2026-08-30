use std::time::Duration;

use pretty_assertions::assert_eq;

use super::FsmonitorOverride;
use super::GitClient;
use super::GitExecutionLimits;
use super::GitInvocation;
use super::REPOSITORY_SELECTOR_ENVIRONMENT;
use crate::GitError;
use crate::test_support::TestRepository;

#[test]
fn execution_limits_reject_zero_values() {
    let error = GitExecutionLimits::new(Duration::ZERO, Duration::from_secs(1), 1)
        .expect_err("zero query timeout");
    assert!(matches!(
        error,
        GitError::InvalidConfiguration {
            field: "query_timeout",
            requirement: "must be non-zero",
        }
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn query_runner_captures_system_git_output() {
    let repository = TestRepository::init();
    let output = GitClient::system()
        .run_query(repository.root(), ["--version"])
        .await
        .expect("run git version");

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .expect("version is UTF-8")
            .starts_with("git version ")
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn git_commands_remove_inherited_repository_selectors() {
    let client = GitClient::system();
    let invocation = GitInvocation::query(
        std::path::Path::new("/dir"),
        ["status"],
        FsmonitorOverride::Disabled,
    );
    let (command, _) = client.configure_command(&invocation);
    let environment = command
        .as_std()
        .get_envs()
        .collect::<std::collections::HashMap<_, _>>();

    for name in REPOSITORY_SELECTOR_ENVIRONMENT {
        assert_eq!(environment.get(std::ffi::OsStr::new(name)), Some(&None));
    }
}
