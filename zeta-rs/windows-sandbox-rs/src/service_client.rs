//! Authenticated client for the installed Zeta Windows sandbox service.

use crate::SANDBOX_SERVICE_NAME;
use crate::SANDBOX_SERVICE_PIPE_NAME;
use crate::SANDBOX_SERVICE_PROTOCOL_VERSION;
use crate::WindowsSandboxProvisioningFrame;
use crate::WindowsSandboxProvisioningMessage;
use crate::WindowsSandboxProvisioningRequest;
use crate::WindowsSandboxProvisioningResponse;
use crate::read_provisioning_frame;
use crate::write_provisioning_frame;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::ffi::OsStr;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::mem::size_of;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::ptr;
use std::time::Duration;
use std::time::Instant;
use windows_sys::Win32::Foundation::ERROR_BROKEN_PIPE;
use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::Foundation::ERROR_NO_DATA;
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;
use windows_sys::Win32::Foundation::ERROR_PIPE_NOT_CONNECTED;
use windows_sys::Win32::Foundation::ERROR_SEM_TIMEOUT;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::SECURITY_IMPERSONATION;
use windows_sys::Win32::Storage::FileSystem::SECURITY_SQOS_PRESENT;
use windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows_sys::Win32::System::Pipes::PeekNamedPipe;
use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
use windows_sys::Win32::System::Services;

const PROVISIONING_TIMEOUT: Duration = Duration::from_secs(125);
const SERVICE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_FRAME_BYTES: usize = 4096;

pub(crate) fn provision(request: &WindowsSandboxProvisioningRequest) -> Result<(), String> {
    provision_inner(request).map_err(|error| format!("{error:#}"))
}

fn provision_inner(request: &WindowsSandboxProvisioningRequest) -> Result<()> {
    let mut pipe = connect(Instant::now() + SERVICE_CONNECT_TIMEOUT)?
        .context("Zeta Windows sandbox service is not installed")?;
    verify_server(pipe.as_raw_handle() as HANDLE)?;
    write_provisioning_frame(
        &mut pipe,
        &WindowsSandboxProvisioningFrame {
            version: SANDBOX_SERVICE_PROTOCOL_VERSION,
            message: WindowsSandboxProvisioningMessage::Provision(request.clone()),
        },
    )
    .context("send Windows sandbox provisioning request")?;
    wait_for_frame(&pipe, Instant::now() + PROVISIONING_TIMEOUT)?;
    let response = read_provisioning_frame(&mut pipe)
        .context("read Windows sandbox provisioning response")?
        .context("Windows sandbox service closed without a response")?;
    if response.version != SANDBOX_SERVICE_PROTOCOL_VERSION {
        bail!(
            "Windows sandbox service uses protocol version {}, expected {}",
            response.version,
            SANDBOX_SERVICE_PROTOCOL_VERSION
        );
    }
    match response.message {
        WindowsSandboxProvisioningMessage::Result(WindowsSandboxProvisioningResponse::Ok) => Ok(()),
        WindowsSandboxProvisioningMessage::Result(WindowsSandboxProvisioningResponse::Error {
            message,
        }) => bail!("Windows sandbox service rejected provisioning: {message}"),
        WindowsSandboxProvisioningMessage::Provision(_) => {
            bail!("Windows sandbox service returned a request instead of a response")
        }
    }
}

fn connect(deadline: Instant) -> Result<Option<File>> {
    let open_pipe = || {
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(SECURITY_SQOS_PRESENT | SECURITY_IMPERSONATION)
            .open(SANDBOX_SERVICE_PIPE_NAME)
    };
    let pipe_name = crate::appcontainer::to_wide(OsStr::new(SANDBOX_SERVICE_PIPE_NAME));
    loop {
        match open_pipe() {
            Ok(pipe) => return Ok(Some(pipe)),
            Err(error) if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) => {
                if foreground_service_enabled() && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(20));
                    continue;
                }
                return Ok(None);
            }
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(None);
                }
                let wait_ms = u32::try_from(remaining.as_millis())
                    .unwrap_or(u32::MAX)
                    .max(1);
                if unsafe { WaitNamedPipeW(pipe_name.as_ptr(), wait_ms) } == 0 {
                    let error = io::Error::last_os_error();
                    if matches!(
                        error.raw_os_error(),
                        Some(code)
                            if code == ERROR_FILE_NOT_FOUND as i32
                                || code == ERROR_SEM_TIMEOUT as i32
                    ) {
                        return Ok(None);
                    }
                    return Err(error).context("wait for Windows sandbox service");
                }
            }
            Err(error) => return Err(error).context("connect to Windows sandbox service"),
        }
    }
}

fn wait_for_frame(pipe: &File, deadline: Instant) -> Result<()> {
    loop {
        let mut prefix = [0_u8; size_of::<u32>()];
        let mut prefix_length = 0;
        let mut available = 0;
        if unsafe {
            PeekNamedPipe(
                pipe.as_raw_handle() as HANDLE,
                prefix.as_mut_ptr().cast(),
                prefix.len() as u32,
                &mut prefix_length,
                &mut available,
                ptr::null_mut(),
            )
        } == 0
        {
            let error = io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(code)
                    if code == ERROR_BROKEN_PIPE as i32
                        || code == ERROR_NO_DATA as i32
                        || code == ERROR_PIPE_NOT_CONNECTED as i32
            ) {
                bail!("Windows sandbox service closed before replying");
            }
            return Err(error).context("inspect Windows sandbox provisioning response");
        }
        if prefix_length as usize == prefix.len() {
            let payload_length = u32::from_le_bytes(prefix) as usize;
            if payload_length > MAX_FRAME_BYTES {
                bail!("Windows sandbox service response exceeds its size limit");
            }
            if available as usize >= prefix.len() + payload_length {
                return Ok(());
            }
        }
        if Instant::now() >= deadline {
            bail!("Windows sandbox service response timed out");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn verify_server(pipe: HANDLE) -> Result<()> {
    if foreground_service_enabled() {
        return Ok(());
    }
    let mut pipe_process_id = 0;
    if unsafe { GetNamedPipeServerProcessId(pipe, &mut pipe_process_id) } == 0 {
        return Err(io::Error::last_os_error()).context("identify Windows sandbox service process");
    }
    let manager =
        unsafe { Services::OpenSCManagerW(ptr::null(), ptr::null(), Services::SC_MANAGER_CONNECT) };
    if manager.is_null() {
        return Err(io::Error::last_os_error()).context("open Windows service manager");
    }
    let manager = ServiceHandle(manager);
    let service_name = crate::appcontainer::to_wide(OsStr::new(SANDBOX_SERVICE_NAME));
    let service = unsafe {
        Services::OpenServiceW(
            manager.0,
            service_name.as_ptr(),
            Services::SERVICE_QUERY_STATUS,
        )
    };
    if service.is_null() {
        return Err(io::Error::last_os_error()).context("open Zeta sandbox service");
    }
    let service = ServiceHandle(service);
    let mut status: Services::SERVICE_STATUS_PROCESS = unsafe { std::mem::zeroed() };
    let mut bytes_needed = 0;
    if unsafe {
        Services::QueryServiceStatusEx(
            service.0,
            Services::SC_STATUS_PROCESS_INFO,
            ptr::from_mut(&mut status).cast(),
            size_of::<Services::SERVICE_STATUS_PROCESS>() as u32,
            &mut bytes_needed,
        )
    } == 0
    {
        return Err(io::Error::last_os_error()).context("query Zeta sandbox service");
    }
    if status.dwCurrentState != Services::SERVICE_RUNNING
        || status.dwProcessId == 0
        || status.dwProcessId != pipe_process_id
    {
        bail!("sandbox provisioning pipe is not owned by the running Zeta sandbox service");
    }
    Ok(())
}

fn foreground_service_enabled() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var("ZETA_WINDOWS_SANDBOX_SERVICE_FOREGROUND").as_deref() == Ok("1")
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}

struct ServiceHandle(Services::SC_HANDLE);

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { Services::CloseServiceHandle(self.0) };
        }
    }
}
