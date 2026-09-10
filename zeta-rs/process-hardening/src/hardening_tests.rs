use super::*;

#[test]
fn loader_keys_preserve_unrelated_environment() {
    for key in ["LD_PRELOAD", "LD_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES"] {
        assert!(dangerous_key(std::ffi::OsStr::new(key)));
    }
    for key in ["PATH", "HOME", "BUILD_ID"] {
        assert!(!dangerous_key(std::ffi::OsStr::new(key)));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        assert!(dangerous_key(std::ffi::OsStr::from_bytes(b"LD_\xff")));
    }
}

#[test]
fn child_process_has_no_loader_environment_or_core_dumps() {
    const CHILD: &str = "ZETA_HARDENING_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        assert!(std::env::var_os("LD_ZETA_TEST").is_none());
        assert!(std::env::var_os("DYLD_ZETA_TEST").is_none());
        initialize().unwrap();
        #[cfg(unix)]
        {
            let mut limit = libc::rlimit {
                rlim_cur: 1,
                rlim_max: 1,
            };
            // SAFETY: writable initialized rlimit for the current process.
            assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) }, 0);
            assert_eq!((limit.rlim_cur, limit.rlim_max), (0, 0));
        }
        #[cfg(target_os = "linux")]
        // SAFETY: PR_GET_DUMPABLE takes no pointers.
        assert_eq!(unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) }, 0);
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::child_process_has_no_loader_environment_or_core_dumps",
        ])
        .env(CHILD, "1")
        .env("LD_ZETA_TEST", "remove")
        .env("DYLD_ZETA_TEST", "remove")
        .status()
        .unwrap();
    assert!(status.success());
}
