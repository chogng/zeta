//! Binds provisioning to the packaged Zeta command runner and its Windows user.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use sha2::Digest;
use sha2::Sha256;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::path::Prefix;
use std::ptr;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Security;
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::System::Pipes;
use windows_sys::Win32::System::Threading;
use zeta_windows_sandbox::WindowsSandboxProvisioningAccess;
use zeta_windows_sandbox::WindowsSandboxProvisioningRequest;

const COMMAND_RUNNER_NAME: &str = "zeta-command-runner.exe";
const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOKEN_USER_BYTES: usize = 4096;
const DRIVE_FIXED: u32 = 3;

pub(crate) struct AuthorizedClientProcess {
    handle: OwnedHandle,
}

pub(crate) struct AuthenticatedRequest {
    pub(crate) request: WindowsSandboxProvisioningRequest,
    _pinned_paths: Vec<OwnedHandle>,
}

pub(crate) struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != Foundation::INVALID_HANDLE_VALUE {
            unsafe { Foundation::CloseHandle(self.0) };
        }
    }
}

pub(crate) fn authorize_client_process(pipe: HANDLE) -> Result<AuthorizedClientProcess> {
    let mut process_id = 0;
    if unsafe { Pipes::GetNamedPipeClientProcessId(pipe, &mut process_id) } == 0 || process_id == 0
    {
        return Err(std::io::Error::last_os_error()).context("identify sandbox command runner");
    }
    let process = unsafe {
        Threading::OpenProcess(Threading::PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id)
    };
    if process.is_null() {
        return Err(std::io::Error::last_os_error()).context("open sandbox command runner");
    }
    let process = OwnedHandle(process);
    let client_path = process_image_path(process.0)?;
    if !client_path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.eq_ignore_ascii_case(COMMAND_RUNNER_NAME))
    {
        bail!("sandbox command runner has an unexpected executable name");
    }
    let trusted_path = std::env::current_exe()
        .context("locate the sandbox service executable")?
        .with_file_name(COMMAND_RUNNER_NAME);
    let trusted_digest = executable_digest(&trusted_path)
        .with_context(|| format!("hash trusted command runner {}", trusted_path.display()))?;
    let client_digest = executable_digest(&client_path)
        .with_context(|| format!("hash command runner {}", client_path.display()))?;
    if trusted_digest != client_digest {
        bail!("sandbox command runner does not match the protected Zeta installation");
    }
    Ok(AuthorizedClientProcess { handle: process })
}

pub(crate) fn authenticate_request(
    pipe: HANDLE,
    process: &AuthorizedClientProcess,
    request: WindowsSandboxProvisioningRequest,
) -> Result<AuthenticatedRequest> {
    let mut impersonation = ImpersonationGuard::begin(pipe)?;
    verify_process_user(process.handle.0)?;

    let mut pinned_paths = Vec::new();
    let dir = validate_and_pin_directory(&request.dir, request.access, &mut pinned_paths)?;
    let program = validate_and_pin_program(&request.program, &mut pinned_paths)?;
    impersonation.revert()?;

    Ok(AuthenticatedRequest {
        request: WindowsSandboxProvisioningRequest {
            dir,
            program,
            access: request.access,
        },
        _pinned_paths: pinned_paths,
    })
}

fn verify_process_user(process: HANDLE) -> Result<()> {
    let mut process_token = ptr::null_mut();
    if unsafe { Threading::OpenProcessToken(process, Security::TOKEN_QUERY, &mut process_token) }
        == 0
    {
        return Err(std::io::Error::last_os_error()).context("open command runner process token");
    }
    let process_token = OwnedHandle(process_token);

    let mut thread_token = ptr::null_mut();
    if unsafe {
        Threading::OpenThreadToken(
            Threading::GetCurrentThread(),
            Security::TOKEN_QUERY,
            1,
            &mut thread_token,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("open command runner pipe token");
    }
    let thread_token = OwnedHandle(thread_token);
    let process_user = token_user(process_token.0)?;
    let thread_user = token_user(thread_token.0)?;
    let process_sid = unsafe {
        ptr::read_unaligned(process_user.as_ptr().cast::<Security::TOKEN_USER>())
            .User
            .Sid
    };
    let thread_sid = unsafe {
        ptr::read_unaligned(thread_user.as_ptr().cast::<Security::TOKEN_USER>())
            .User
            .Sid
    };
    if process_sid.is_null()
        || thread_sid.is_null()
        || unsafe { Security::EqualSid(process_sid, thread_sid) } == 0
    {
        bail!("command runner process does not belong to the pipe user");
    }
    Ok(())
}

fn token_user(token: HANDLE) -> Result<Vec<u8>> {
    let mut length = 0;
    unsafe {
        Security::GetTokenInformation(token, Security::TokenUser, ptr::null_mut(), 0, &mut length)
    };
    if length < size_of::<Security::TOKEN_USER>() as u32 || length as usize > MAX_TOKEN_USER_BYTES {
        bail!("Windows token returned an invalid user identity length");
    }
    let mut buffer = vec![0_u8; length as usize];
    if unsafe {
        Security::GetTokenInformation(
            token,
            Security::TokenUser,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("read Windows token user");
    }
    if length < size_of::<Security::TOKEN_USER>() as u32 || length as usize > buffer.len() {
        bail!("Windows token returned a malformed user identity");
    }
    Ok(buffer)
}

fn validate_and_pin_directory(
    path: &Path,
    access: WindowsSandboxProvisioningAccess,
    handles: &mut Vec<OwnedHandle>,
) -> Result<PathBuf> {
    validate_absolute_local_path(path)?;
    pin_existing_ancestors(path, handles)?;
    let desired_access = FileSystem::READ_CONTROL
        | FileSystem::WRITE_DAC
        | FileSystem::FILE_READ_ATTRIBUTES
        | FileSystem::FILE_TRAVERSE
        | match access {
            WindowsSandboxProvisioningAccess::ReadOnly => 0,
            WindowsSandboxProvisioningAccess::DirectoryWrite => {
                FileSystem::FILE_ADD_FILE | FileSystem::FILE_ADD_SUBDIRECTORY
            }
        };
    handles.push(open_path(path, desired_access, true)?);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalize sandbox directory {}", path.display()))?;
    validate_absolute_local_path(&canonical)?;
    Ok(canonical)
}

fn validate_and_pin_program(path: &Path, handles: &mut Vec<OwnedHandle>) -> Result<PathBuf> {
    validate_absolute_local_path(path)?;
    let parent = path
        .parent()
        .context("sandboxed program has no parent directory")?;
    pin_existing_ancestors(parent, handles)?;
    handles.push(open_path(
        path,
        Foundation::GENERIC_READ | Foundation::GENERIC_EXECUTE | FileSystem::WRITE_DAC,
        false,
    )?);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalize sandboxed program {}", path.display()))?;
    validate_absolute_local_path(&canonical)?;
    Ok(canonical)
}

fn validate_absolute_local_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.as_os_str().is_empty() {
        bail!("sandbox provisioning paths must be absolute");
    }
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        bail!("sandbox provisioning paths must begin with a local drive");
    };
    if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
        bail!("sandbox provisioning paths must use a local drive");
    }
    for component in components {
        if matches!(component, Component::ParentDir) {
            bail!("sandbox provisioning paths cannot contain parent traversal");
        }
    }
    let root = path
        .ancestors()
        .last()
        .context("sandbox provisioning path has no drive root")?;
    if unsafe { FileSystem::GetDriveTypeW(to_wide(root.as_os_str()).as_ptr()) } != DRIVE_FIXED {
        bail!("sandbox provisioning paths must use a fixed local drive");
    }
    Ok(())
}

fn pin_existing_ancestors(path: &Path, handles: &mut Vec<OwnedHandle>) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.is_absolute() && !matches!(component, Component::Prefix(_)) {
            handles.push(open_path(
                &current,
                FileSystem::FILE_READ_ATTRIBUTES | FileSystem::FILE_TRAVERSE,
                true,
            )?);
        }
    }
    Ok(())
}

fn open_path(path: &Path, desired_access: u32, directory: bool) -> Result<OwnedHandle> {
    let metadata = path
        .symlink_metadata()
        .with_context(|| format!("inspect provisioning path {}", path.display()))?;
    if metadata.file_attributes() & FileSystem::FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        bail!("refusing reparse point {}", path.display());
    }
    if directory != metadata.is_dir() {
        bail!(
            "provisioning path has the wrong file type: {}",
            path.display()
        );
    }
    let flags = FileSystem::FILE_FLAG_OPEN_REPARSE_POINT
        | if directory {
            FileSystem::FILE_FLAG_BACKUP_SEMANTICS
        } else {
            0
        };
    let handle = unsafe {
        FileSystem::CreateFileW(
            to_wide(path.as_os_str()).as_ptr(),
            desired_access,
            FileSystem::FILE_SHARE_READ | FileSystem::FILE_SHARE_WRITE,
            ptr::null(),
            FileSystem::OPEN_EXISTING,
            flags,
            ptr::null_mut(),
        )
    };
    if handle == Foundation::INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("pin provisioning path {}", path.display()));
    }
    Ok(OwnedHandle(handle))
}

fn process_image_path(process: HANDLE) -> Result<PathBuf> {
    let mut buffer = vec![0_u16; 32_768];
    let mut length = buffer.len() as u32;
    if unsafe {
        Threading::QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length)
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("read command runner image path");
    }
    buffer.truncate(length as usize);
    Ok(PathBuf::from(String::from_utf16(&buffer)?))
}

fn executable_digest(path: &Path) -> Result<[u8; 32]> {
    let metadata = path
        .symlink_metadata()
        .with_context(|| format!("inspect executable {}", path.display()))?;
    if !metadata.is_file()
        || metadata.file_attributes() & FileSystem::FILE_ATTRIBUTE_REPARSE_POINT != 0
        || metadata.len() > MAX_EXECUTABLE_BYTES
    {
        bail!("refusing invalid executable {}", path.display());
    }
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

struct ImpersonationGuard {
    active: bool,
}

impl ImpersonationGuard {
    fn begin(pipe: HANDLE) -> Result<Self> {
        if unsafe { Pipes::ImpersonateNamedPipeClient(pipe) } == 0 {
            return Err(std::io::Error::last_os_error()).context("impersonate command runner");
        }
        Ok(Self { active: true })
    }

    fn revert(&mut self) -> Result<()> {
        if self.active {
            if unsafe { Security::RevertToSelf() } == 0 {
                return Err(std::io::Error::last_os_error())
                    .context("stop impersonating command runner");
            }
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for ImpersonationGuard {
    fn drop(&mut self) {
        if self.active && unsafe { Security::RevertToSelf() } == 0 {
            std::process::abort();
        }
    }
}
