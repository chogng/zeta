use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{FreeSid, PSID};

const HRESULT_ALREADY_EXISTS: i32 = 0x8007_00B7u32 as i32;

pub(crate) struct AppContainerSid(PSID);

impl AppContainerSid {
    pub(crate) fn ensure(profile: &OsStr) -> Result<Self, String> {
        let profile = to_wide(profile);
        let display = to_wide(OsStr::new("Zeta Agent Sandbox"));
        let description = to_wide(OsStr::new("Zeta local tool AppContainer"));
        let mut sid = std::ptr::null_mut();
        let created = unsafe {
            CreateAppContainerProfile(
                profile.as_ptr(),
                display.as_ptr(),
                description.as_ptr(),
                std::ptr::null(),
                0,
                &mut sid,
            )
        };
        if created >= 0 {
            return Ok(Self(sid));
        }
        if created != HRESULT_ALREADY_EXISTS {
            return Err(format!(
                "CreateAppContainerProfile failed with HRESULT 0x{:08x}",
                created as u32
            ));
        }
        let derived =
            unsafe { DeriveAppContainerSidFromAppContainerName(profile.as_ptr(), &mut sid) };
        if derived < 0 {
            return Err(format!(
                "DeriveAppContainerSidFromAppContainerName failed with HRESULT 0x{:08x}",
                derived as u32
            ));
        }
        Ok(Self(sid))
    }

    pub(crate) fn as_ptr(&self) -> PSID {
        self.0
    }
}

impl Drop for AppContainerSid {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                FreeSid(self.0);
            }
        }
    }
}

pub(crate) struct OwnedHandle(HANDLE);

impl OwnedHandle {
    pub(crate) fn new(handle: HANDLE, operation: &str) -> Result<Self, String> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            Err(last_error(operation))
        } else {
            Ok(Self(handle))
        }
    }

    pub(crate) fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

pub(crate) fn canonical_file(path: &Path, description: &str) -> Result<PathBuf, String> {
    let canonical =
        std::fs::canonicalize(path).map_err(|error| format!("{description}: {error}"))?;
    if !canonical.is_file() {
        return Err(format!("{description} is not a regular file"));
    }
    Ok(canonical)
}

pub(crate) fn canonical_directory(path: &Path, description: &str) -> Result<PathBuf, String> {
    let canonical =
        std::fs::canonicalize(path).map_err(|error| format!("{description}: {error}"))?;
    if !canonical.is_dir() {
        return Err(format!("{description} is not a directory"));
    }
    Ok(canonical)
}

pub(crate) fn last_error(operation: &str) -> String {
    format!("{operation} failed with Windows error {}", unsafe {
        GetLastError()
    })
}

pub(crate) fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

pub(crate) fn command_line(arguments: &[OsString]) -> Vec<u16> {
    let mut command = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if index != 0 {
            command.push(b' ' as u16);
        }
        append_quoted(&mut command, &argument.encode_wide().collect::<Vec<_>>());
    }
    command.push(0);
    command
}

fn append_quoted(output: &mut Vec<u16>, argument: &[u16]) {
    let quote = argument.is_empty()
        || argument
            .iter()
            .any(|character| matches!(*character, 0x09 | 0x20 | 0x22));
    if !quote {
        output.extend_from_slice(argument);
        return;
    }
    output.push(b'"' as u16);
    let mut backslashes = 0;
    for character in argument {
        if *character == b'\\' as u16 {
            backslashes += 1;
        } else if *character == b'"' as u16 {
            output.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2 + 1));
            output.push(*character);
            backslashes = 0;
        } else {
            output.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            output.push(*character);
            backslashes = 0;
        }
    }
    output.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    output.push(b'"' as u16);
}
