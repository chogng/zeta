use super::*;
use std::io::Read;

#[path = "trace.rs"]
mod trace;

#[test]
#[ignore = "requires explicitly provisioned Zeta accounts"]
fn provisioned_logon_starts_on_its_private_desktop() {
    let lease = super::super::runtime::lease(super::super::account::NetworkMode::Denied).unwrap();
    let owner = win::current_user().unwrap();
    let desktop =
        super::super::desktop::Desktop::new(&owner, &lease.account.sid, "S-1-5-21-911-912-913-914")
            .unwrap();
    let mut desktop_name = win::wide(&desktop.name);
    let startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        lpDesktop: desktop_name.as_mut_ptr(),
        ..unsafe { std::mem::zeroed() }
    };
    let system = std::env::var("SystemRoot").unwrap();
    let executable = format!("{system}\\System32\\cmd.exe");
    let mut command = win::wide(format!("\"{executable}\" /d /c exit 0"));
    let mut password = win::wide(&lease.account.password);
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let result = unsafe {
        CreateProcessWithLogonW(
            win::wide(&lease.account.name).as_ptr(),
            win::wide(".").as_ptr(),
            password.as_ptr(),
            0,
            win::wide(&executable).as_ptr(),
            command.as_mut_ptr(),
            CREATE_SUSPENDED | CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT,
            std::ptr::null(),
            win::wide(&system).as_ptr(),
            &startup,
            &mut info,
        )
    };
    for value in &mut password {
        unsafe {
            std::ptr::write_volatile(value, 0);
        }
    }
    assert_ne!(
        result,
        0,
        "{}",
        win::error("CreateProcessWithLogonW(system directory)")
    );
    let process = Handle::new(info.hProcess, "test logon child").unwrap();
    let thread = Handle::new(info.hThread, "test logon thread").unwrap();
    let job = super::super::job::Job::new(&win::random_hex(16).unwrap()).unwrap();
    if let Err(error) = job.assign_process(process.0) {
        unsafe {
            TerminateProcess(process.0, 1);
        }
        panic!("{error}");
    }
    assert_eq!(unsafe { ResumeThread(thread.0) }, 1);
    assert_eq!(
        unsafe { WaitForSingleObject(process.0, 10000) },
        WAIT_OBJECT_0
    );
    let mut code = 0;
    assert_ne!(unsafe { GetExitCodeProcess(process.0, &mut code) }, 0);
    assert_eq!(code, 0);
}

#[test]
#[ignore = "requires explicitly provisioned Zeta accounts"]
fn provisioned_account_can_read_bootstrap_and_working_directories() {
    let lease = super::super::runtime::lease(super::super::account::NetworkMode::Denied).unwrap();
    let directory = super::super::Directory::new(
        &lease.root,
        &win::current_user().unwrap(),
        &lease.account.sid,
        "S-1-5-21-811-812-813-814",
    )
    .unwrap();
    let mut password = win::wide(&lease.account.password);
    let mut raw = std::ptr::null_mut();
    let result = unsafe {
        LogonUserW(
            win::wide(&lease.account.name).as_ptr(),
            win::wide(".").as_ptr(),
            password.as_ptr(),
            LOGON32_LOGON_INTERACTIVE,
            LOGON32_PROVIDER_DEFAULT,
            &mut raw,
        )
    };
    for value in &mut password {
        unsafe {
            std::ptr::write_volatile(value, 0);
        }
    }
    assert_ne!(result, 0, "{}", win::error("LogonUserW"));
    let token = Handle::new(raw, "test logon token").unwrap();
    assert_eq!(win::token_user(token.0).unwrap(), lease.account.sid);
    struct Revert;
    impl Drop for Revert {
        fn drop(&mut self) {
            assert_ne!(unsafe { RevertToSelf() }, 0);
        }
    }
    assert_ne!(
        unsafe { ImpersonateLoggedOnUser(token.0) },
        0,
        "{}",
        win::error("ImpersonateLoggedOnUser")
    );
    let _revert = Revert;
    std::fs::metadata(&lease.runner).expect("sandbox user can read bootstrap executable metadata");
    std::fs::File::open(&lease.runner)
        .expect("sandbox user can read bootstrap executable contents");
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&lease.runner)
            .is_err(),
        "sandbox user must not modify the trusted bootstrap"
    );
    assert!(
        std::fs::File::open(lease.root.join("state.dpapi")).is_err(),
        "sandbox user must not read the credential journal"
    );
    assert!(
        std::fs::File::open(lease.root.join("lease-1")).is_err(),
        "sandbox user must not read or acquire host leases"
    );
    std::fs::read_dir(lease.runner.parent().unwrap())
        .expect("sandbox user can list bootstrap directory");
    std::fs::read_dir(&directory.0).expect("sandbox user can list its execution directory");
}

#[test]
fn restricted_child_uses_private_desktop_pipes_and_preserves_exit_code() {
    let system = std::env::var("SystemRoot").unwrap();
    check_child(format!(
        "\"{system}\\System32\\cmd.exe\" /d /c \"echo child-ready & exit /b 125\""
    ));
}

#[test]
#[ignore = "requires explicitly installed CNG, KsecDD and Null device capability"]
fn powershell_initializes_and_runs_a_pipeline_with_the_restricted_token() {
    let system = std::env::var("SystemRoot").unwrap();
    check_child(format!(
        "\"{system}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoLogo -NoProfile -NonInteractive -Command \"'child-ready' | ForEach-Object {{ Write-Output $_ }}; exit 125\""
    ));
}

fn check_child(command: String) {
    // Exercises the actual child-creation path under the current ordinary user.
    // Dedicated-account logon and network enforcement are separate acceptance.
    let temp = tempfile::tempdir().unwrap();
    let directory = std::fs::canonicalize(temp.path()).unwrap();
    let _directory_pin = super::super::filesystem::pin(&directory).unwrap();
    let owner = win::current_user().unwrap();
    let capability = "S-1-5-21-511-512-513-514";
    let desktop = super::super::desktop::Desktop::new(&owner, &owner, capability).unwrap();
    let pipes = Pipes::new(&owner, &owner).unwrap();
    let system = std::env::var("SystemRoot").unwrap();
    let request = Request {
        version: 1,
        owner: owner.clone(),
        account: owner,
        capability: capability.into(),
        device_capability: super::super::runtime::device_sid().unwrap(),
        parent_logon: win::current_logon().unwrap(),
        command,
        cwd: directory.to_str().unwrap().into(),
        environment: vec![
            format!("SystemRoot={system}"),
            format!("TEMP={}", directory.display()),
        ],
        pipes: pipes.names.clone(),
        reply: directory.join("unused-reply.json"),
        desktop: desktop.name.clone(),
    };
    let (process, thread, _, _) = prepare_child(&request).unwrap();
    struct Cleanup(HANDLE);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            unsafe {
                TerminateProcess(self.0, 1);
                WaitForSingleObject(self.0, 5000);
            }
        }
    }
    let _cleanup = Cleanup(process.0);
    let job = super::super::job::Job::new(&win::random_hex(16).unwrap()).unwrap();
    job.assign_process(process.0).unwrap();
    pipes.connect(unsafe { GetCurrentProcess() }).unwrap();
    let [stdin, mut stdout, mut stderr] = pipes.files();
    drop(stdin);
    assert_eq!(unsafe { ResumeThread(thread.0) }, 1);
    if std::env::var_os("ZETA_TRACE_DENIALS").is_some() {
        trace::run(process.0);
    }
    assert_eq!(
        unsafe { WaitForSingleObject(process.0, 10000) },
        WAIT_OBJECT_0
    );
    let mut code = 0;
    assert_ne!(unsafe { GetExitCodeProcess(process.0, &mut code) }, 0);
    let mut output = Vec::new();
    let mut errors = Vec::new();
    stdout.read_to_end(&mut output).unwrap();
    stderr.read_to_end(&mut errors).unwrap();
    let output = String::from_utf8_lossy(&output);
    let errors = String::from_utf8_lossy(&errors);
    assert_eq!(code, 125, "{output}\n{errors}");
    assert!(output.contains("child-ready"), "{output}\n{errors}");
}
