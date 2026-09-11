//! Installation-owned device permissions, separate from command filesystem authority.
use super::win;
use super::win::Result;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::*;

pub(super) const PATHS: [&str; 3] = [
    r"\\?\GLOBALROOT\Device\CNG",
    r"\\?\GLOBALROOT\Device\KsecDD",
    r"\\?\GLOBALROOT\Device\Null",
];
const ACCESS: u32 = FILE_GENERIC_READ | FILE_GENERIC_WRITE;

fn open(path: &str, access: u32) -> Result<win::Handle> {
    win::Handle::new(
        unsafe {
            CreateFileW(
                win::wide(path).as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        },
        "open sandbox device",
    )
}

pub(super) fn verify(sid: &str) -> Result<()> {
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    use windows_sys::Win32::System::Threading::OpenProcessToken;
    let sid = win::sid(sid)?;
    let mut raw = std::ptr::null_mut();
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_IMPERSONATE,
            &mut raw,
        )
    } == 0
    {
        return Err(win::error("OpenProcessToken(device probe)"));
    }
    let base = win::Handle::new(raw, "device probe token")?;
    let group = SID_AND_ATTRIBUTES {
        Sid: sid.0,
        Attributes: 0,
    };
    let mut raw = std::ptr::null_mut();
    if unsafe {
        CreateRestrictedToken(
            base.0,
            DISABLE_MAX_PRIVILEGE | WRITE_RESTRICTED,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &group,
            &mut raw,
        )
    } == 0
    {
        return Err(win::error("CreateRestrictedToken(device probe)"));
    }
    let restricted = win::Handle::new(raw, "restricted device probe token")?;
    if unsafe { ImpersonateLoggedOnUser(restricted.0) } == 0 {
        return Err(win::error("ImpersonateLoggedOnUser(device probe)"));
    }
    struct Revert;
    impl Drop for Revert {
        fn drop(&mut self) {
            unsafe {
                if RevertToSelf() == 0 {
                    std::process::abort();
                }
            }
        }
    }
    let _revert = Revert;
    for path in PATHS {
        open(path, ACCESS).map_err(|error| format!("device capability unavailable on {path}: {error}; explicit installation refresh is required"))?;
    }
    Ok(())
}

pub(super) fn install(sid: &str, before_change: impl FnMut(&str) -> Result<()>) -> Result<()> {
    update(sid, SET_ACCESS, &PATHS.map(str::to_owned), before_change)
}
pub(super) fn remove(sid: &str, paths: &[String]) -> Result<()> {
    update(sid, REVOKE_ACCESS, paths, |_| Ok(()))
}

fn update(
    sid: &str,
    mode: ACCESS_MODE,
    paths: &[String],
    mut before_change: impl FnMut(&str) -> Result<()>,
) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    if paths.iter().any(|path| !PATHS.contains(&path.as_str())) {
        return Err("unrecognized device in installation journal".into());
    }
    use windows_sys::Win32::Foundation::WAIT_ABANDONED;
    use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
    use windows_sys::Win32::System::Threading::*;
    let descriptor = win::descriptor("D:P(A;;GA;;;SY)(A;;GA;;;BA)")?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let handle = win::Handle::new(
        unsafe {
            CreateMutexW(
                &attributes,
                0,
                win::wide("Global\\ZetaSandbox.Devices").as_ptr(),
            )
        },
        "CreateMutexW(device configuration)",
    )?;
    if !matches!(
        unsafe { WaitForSingleObject(handle.0, 10000) },
        WAIT_OBJECT_0 | WAIT_ABANDONED
    ) {
        return Err("another device configuration operation has not completed".into());
    }
    struct Lock(win::Handle);
    impl Drop for Lock {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0.0);
            }
        }
    }
    let _lock = Lock(handle);
    let sid = win::sid(sid)?;
    for path in paths {
        let handle = open(path, READ_CONTROL | WRITE_DAC)?;
        let mut descriptor = std::ptr::null_mut();
        let mut acl = std::ptr::null_mut();
        let result = unsafe {
            GetSecurityInfo(
                handle.0,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut acl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if result != 0 {
            return Err(format!(
                "read device ACL for {path}: Windows error {result}"
            ));
        }
        let _descriptor = win::Local(descriptor);
        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: ACCESS,
            grfAccessMode: mode,
            grfInheritance: 0,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: std::ptr::null_mut(),
                MultipleTrusteeOperation: 0,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: sid.0.cast(),
            },
        };
        let mut updated = std::ptr::null_mut();
        let result = unsafe { SetEntriesInAclW(1, &entry, acl, &mut updated) };
        if result != 0 {
            return Err(format!(
                "prepare device ACL for {path}: Windows error {result}"
            ));
        }
        let _updated = win::Local(updated.cast());
        before_change(path)?;
        let result = unsafe {
            SetSecurityInfo(
                handle.0,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                updated,
                std::ptr::null_mut(),
            )
        };
        if result != 0 {
            return Err(format!(
                "update device ACL for {path}: Windows error {result}"
            ));
        }
    }
    Ok(())
}
