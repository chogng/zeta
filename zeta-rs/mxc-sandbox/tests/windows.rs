//! Windows acceptance uses the executor → adapter → MXC SDK → ProcessContainer chain.
#![cfg(windows)]

use mxc_sandbox::MxcSandbox;
use network_proxy::NetworkDecision;
use network_proxy::NetworkPolicyHandle;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use zeta_async_utils::CancellationSource;
use zeta_file_access::Dir;
use zeta_install_context::InstallContext;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxBackend;
use zeta_sandboxing::SandboxDirAccess;
use zeta_sandboxing::SandboxDirGrant;
use zeta_sandboxing::SandboxKind;
use zeta_sandboxing::SandboxPolicy;
use zeta_sandboxing::SandboxScope;
use zeta_tool_executor::ApprovalPolicy;
use zeta_tool_executor::ApprovalRequirement;
use zeta_tool_executor::CommandExecutionAuthority;
use zeta_tool_executor::CommandExecutionOutcome;
use zeta_tool_executor::CommandExecutor;
use zeta_tool_executor::CommandInput;
use zeta_tool_executor::CommandRequest;
use zeta_tool_executor::ExecutionError;
use zeta_tool_executor::ExecutionLimits;

struct Approved;
impl ApprovalPolicy for Approved {
    fn requirement_for(&self, _: &str) -> ApprovalRequirement {
        ApprovalRequirement::NotRequired
    }
}

fn executor(dir: &Dir, timeout: Duration) -> CommandExecutor<Approved, MxcSandbox> {
    let backend = MxcSandbox::new(InstallContext::current());
    assert_eq!(
        backend.kind(),
        SandboxKind::Restricted,
        "the adapter must prepare restricted execution"
    );
    assert!(backend.requires_shared_network_proxy());
    CommandExecutor::new(
        dir.clone(),
        backend,
        Approved,
        ExecutionLimits {
            timeout,
            max_output_bytes: 64 * 1024,
        },
    )
}

fn sandbox_policy(files: FileSystemAccess, network: NetworkAccess) -> SandboxPolicy {
    SandboxPolicy::new(files, network)
        .with_host_acl_changes(zeta_sandboxing::HostAclChanges::Scoped)
}

fn powershell(script: String) -> CommandRequest {
    let program = Path::new(&std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    CommandRequest {
        program: program.to_str().unwrap().into(),
        arguments: vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script,
        ],
        working_directory: ".".into(),
        input: CommandInput::Closed,
    }
}

fn literal(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "''"))
}

#[test]
#[ignore = "requires Windows with MXC filesystem enforcement and scoped ACL permissions"]
fn scoped_execution_preserves_grants_metadata_and_exit_code_authenticity() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["work", "reference", "other-agent"] {
        std::fs::create_dir(temp.path().join(name)).unwrap();
    }
    let storage = Dir::open_local(temp.path()).unwrap();
    let work = Dir::open_local(temp.path().join("work")).unwrap();
    let reference = Dir::open_local(temp.path().join("reference")).unwrap();
    let secret = temp.path().join("other-agent/secret");
    std::fs::write(&secret, "secret").unwrap();
    std::fs::write(reference.canonical_path().join("input"), "reference").unwrap();
    std::fs::create_dir(work.canonical_path().join(".git")).unwrap();
    let scope = SandboxScope::new(
        work.clone(),
        vec![
            SandboxDirGrant::new(work.clone(), SandboxDirAccess::ReadWrite),
            SandboxDirGrant::new(reference.clone(), SandboxDirAccess::ReadOnly),
        ],
        vec![storage],
    )
    .unwrap();
    let script = format!(
        "$ErrorActionPreference='Stop'; Set-Content output 'ok'; \
         if ((Get-Content {}) -ne 'reference') {{ exit 10 }}; \
         function MustDeny([scriptblock]$action) {{ try {{ & $action }} catch {{ return }}; throw 'restriction was not enforced' }}; \
         MustDeny {{ Get-Content {} }}; MustDeny {{ Set-Content {} 'bad' }}; \
         MustDeny {{ Set-Content .git/modified 'bad' }}; Write-Output 'scoped-ok'; exit 125",
        literal(&reference.canonical_path().join("input")),
        literal(&secret),
        literal(&reference.canonical_path().join("modified")),
    );
    let result = executor(&work, Duration::from_secs(30))
        .execute_scoped_with_network(
            powershell(script),
            CommandExecutionAuthority::Sandboxed(sandbox_policy(
                FileSystemAccess::DirectoryWrite,
                NetworkAccess::Denied,
            )),
            &CancellationSource::new().token(),
            Some(&scope),
            None,
        )
        .unwrap();
    let CommandExecutionOutcome::Completed(output) = result else {
        panic!("{result:?}")
    };
    assert_eq!(output.exit_code, Some(125), "{output:?}");
    assert!(output.stdout.contains("scoped-ok"), "{output:?}");
    assert!(work.canonical_path().join("output").exists());
    assert!(!reference.canonical_path().join("modified").exists());
    assert!(!work.canonical_path().join(".git/modified").exists());
}

#[test]
fn strict_managed_network_is_rejected_without_weakening_ingress_or_proxy_identity() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Dir::open_local(temp.path()).unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let policy = NetworkPolicyHandle::new(move |_, _| {
        observed.fetch_add(1, Ordering::Relaxed);
        async { NetworkDecision::Allow }
    });
    let result = executor(&dir, Duration::from_secs(5)).execute_scoped_with_network(
        powershell("Set-Content started 'must not run'".into()),
        CommandExecutionAuthority::Sandboxed(sandbox_policy(
            FileSystemAccess::DirectoryWrite,
            NetworkAccess::Managed,
        )),
        &CancellationSource::new().token(),
        None,
        Some(&policy),
    );
    assert!(
        matches!(result, Err(ExecutionError::Sandbox(_))),
        "{result:?}"
    );
    assert!(!dir.canonical_path().join("started").exists());
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
#[ignore = "requires PSEC and the built runner"]
fn timeout_and_cancellation_terminate_psec_descendants() {
    for mode in ["timeout", "cancel"] {
        let temp = tempfile::tempdir().unwrap();
        let dir = Dir::open_local(temp.path()).unwrap();
        let pid_file = dir.canonical_path().join("child.pid");
        let cancellation = CancellationSource::new();
        let watcher = if mode == "cancel" {
            let path = pid_file.clone();
            let source = cancellation.clone();
            Some(std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + Duration::from_secs(12);
                while !path.exists() && std::time::Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(20));
                }
                source.cancel();
            }))
        } else {
            None
        };
        let script = format!(
            "$ErrorActionPreference='Stop'; $p=Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList '-NoProfile','-Command','Start-Sleep 60' -PassThru; [IO.File]::WriteAllText({}, [string]$p.Id); Start-Sleep 60",
            literal(&pid_file)
        );
        let timeout = Duration::from_secs(if mode == "cancel" { 15 } else { 8 });
        let result = executor(&dir, timeout).execute(
            powershell(script),
            CommandExecutionAuthority::Sandboxed(sandbox_policy(
                FileSystemAccess::DirectoryWrite,
                NetworkAccess::Denied,
            )),
            &cancellation.token(),
        );
        if let Some(watcher) = watcher {
            watcher.join().unwrap();
        }
        match (mode, result) {
            ("timeout", Err(ExecutionError::TimedOut))
            | ("cancel", Err(ExecutionError::CancelledAfterStart(_))) => {}
            (_, result) => panic!("unexpected {mode} result: {result:?}"),
        }
        let pid: u32 = std::fs::read_to_string(pid_file)
            .expect("sandbox must start its descendant before termination")
            .parse()
            .unwrap();
        assert_process_exited(pid);
    }
}

fn assert_process_exited(pid: u32) {
    let status = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 1 }}"),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "SDK descendant survived execution");
}
