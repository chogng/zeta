use crate::WindowsSandboxProvisioningAccess;
use crate::WindowsSandboxProvisioningRequest;
use crate::appcontainer::{AppContainerSid, canonical_directory, canonical_file, to_wide};
use crate::profile_name;
use crate::protocol::{DIR_WRITE_ACCESS, READ_ONLY_ACCESS};
use std::ffi::c_void;
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    DENY_ACCESS, EXPLICIT_ACCESS_W, GRANT_ACCESS, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    ACL, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, OBJECT_INHERIT_ACE, PSID,
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_ATTRIBUTE_REPARSE_POINT, FILE_DELETE_CHILD, FILE_GENERIC_EXECUTE,
    FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};

pub(crate) fn provision(request: &WindowsSandboxProvisioningRequest) -> Result<(), String> {
    let dir = canonical_directory(&request.dir, "dir")?;
    let program = canonical_file(&request.program, "sandboxed program")?;
    let program_directory = canonical_directory(
        program
            .parent()
            .ok_or("sandboxed program has no parent directory")?,
        "sandboxed program directory",
    )?;
    let (access, dir_mask) = match request.access {
        WindowsSandboxProvisioningAccess::ReadOnly => {
            (READ_ONLY_ACCESS, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE)
        }
        WindowsSandboxProvisioningAccess::DirectoryWrite => (
            DIR_WRITE_ACCESS,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE | DELETE,
        ),
    };
    let profile = profile_name(&dir, access);
    let sid = AppContainerSid::ensure(std::ffi::OsStr::new(&profile))?;
    add_ace(
        &dir,
        sid.as_ptr(),
        dir_mask,
        GRANT_ACCESS,
        OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
    )?;
    add_ace(
        &program_directory,
        sid.as_ptr(),
        FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        GRANT_ACCESS,
        OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
    )?;
    add_ace(
        &program,
        sid.as_ptr(),
        FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
        GRANT_ACCESS,
        0,
    )?;
    if access == DIR_WRITE_ACCESS {
        for name in zeta_sandboxing::PROTECTED_DIR_METADATA_NAMES {
            let protected = dir.join(name);
            if protected.exists() {
                add_ace_tree(
                    &protected,
                    sid.as_ptr(),
                    FILE_GENERIC_WRITE | FILE_DELETE_CHILD | DELETE,
                    DENY_ACCESS,
                )?;
            }
        }
    }
    Ok(())
}

fn add_ace_tree(path: &Path, sid: PSID, permissions: u32, mode: i32) -> Result<(), String> {
    let metadata = path.symlink_metadata().map_err(|error| {
        format!(
            "could not inspect protected path {}: {error}",
            path.display()
        )
    })?;
    add_ace(
        path,
        sid,
        permissions,
        mode,
        if metadata.is_dir() {
            OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
        } else {
            0
        },
    )?;
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Ok(());
    }
    let entries = std::fs::read_dir(path).map_err(|error| {
        format!(
            "could not traverse protected path {}: {error}",
            path.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not inspect an entry below protected path {}: {error}",
                path.display()
            )
        })?;
        add_ace_tree(&entry.path(), sid, permissions, mode)?;
    }
    Ok(())
}

fn add_ace(
    path: &Path,
    sid: PSID,
    permissions: u32,
    mode: i32,
    inheritance: u32,
) -> Result<(), String> {
    let path_wide = to_wide(path.as_os_str());
    let mut old_dacl: *mut ACL = std::ptr::null_mut();
    let mut security_descriptor: *mut c_void = std::ptr::null_mut();
    let get_result = unsafe {
        GetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut old_dacl,
            std::ptr::null_mut(),
            &mut security_descriptor,
        )
    };
    if get_result != ERROR_SUCCESS {
        return Err(format!(
            "GetNamedSecurityInfoW failed for {}: {get_result}",
            path.display()
        ));
    }
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: permissions,
        grfAccessMode: mode,
        grfInheritance: inheritance,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: 0,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: sid as *mut u16,
        },
    };
    let mut new_dacl: *mut ACL = std::ptr::null_mut();
    let acl_result = unsafe { SetEntriesInAclW(1, &entry, old_dacl, &mut new_dacl) };
    if acl_result != ERROR_SUCCESS {
        unsafe {
            LocalFree(security_descriptor as HLOCAL);
        }
        return Err(format!(
            "SetEntriesInAclW failed for {}: {acl_result}",
            path.display()
        ));
    }
    let set_result = unsafe {
        SetNamedSecurityInfoW(
            path_wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        )
    };
    unsafe {
        LocalFree(new_dacl as HLOCAL);
        LocalFree(security_descriptor as HLOCAL);
    }
    if set_result != ERROR_SUCCESS {
        return Err(format!(
            "SetNamedSecurityInfoW failed for {}: {set_result}",
            path.display()
        ));
    }
    Ok(())
}
