//! Bounded local IPC for authenticated AppContainer provisioning.

use crate::authentication::authenticate_request;
use crate::authentication::authorize_client_process;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Security;
use windows_sys::Win32::Security::Authorization;
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::System::Pipes;
use zeta_windows_sandbox::SANDBOX_SERVICE_PIPE_NAME;
use zeta_windows_sandbox::SANDBOX_SERVICE_PROTOCOL_VERSION;
use zeta_windows_sandbox::WindowsSandboxProvisioningFrame;
use zeta_windows_sandbox::WindowsSandboxProvisioningMessage;
use zeta_windows_sandbox::WindowsSandboxProvisioningResponse;
use zeta_windows_sandbox::read_provisioning_frame;
use zeta_windows_sandbox::write_provisioning_frame;

const MAX_REQUEST_BYTES: usize = 4096;
const MAX_ERROR_BYTES: usize = 512;
const REQUEST_IDLE_TIMEOUT: Duration = Duration::from_secs(5);
const PIPE_USER_ACCESS: &str = "0x0012019b";

struct PipeHandle(HANDLE);

impl Drop for PipeHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != Foundation::INVALID_HANDLE_VALUE {
            unsafe { Foundation::CloseHandle(self.0) };
        }
    }
}

struct SecurityDescriptor(Security::PSECURITY_DESCRIPTOR);

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe { Foundation::LocalFree(self.0 as Foundation::HLOCAL) };
    }
}

pub(crate) fn run(shutdown: Arc<AtomicBool>, on_ready: impl FnOnce() -> Result<()>) -> Result<()> {
    let sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;{PIPE_USER_ACCESS};;;IU)");
    let mut descriptor: Security::PSECURITY_DESCRIPTOR = ptr::null_mut();
    if unsafe {
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW(
            to_wide(OsStr::new(&sddl)).as_ptr(),
            Authorization::SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("create sandbox pipe DACL");
    }
    let descriptor = SecurityDescriptor(descriptor);
    let attributes = Security::SECURITY_ATTRIBUTES {
        nLength: size_of::<Security::SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let pipe = unsafe {
        Pipes::CreateNamedPipeW(
            to_wide(OsStr::new(SANDBOX_SERVICE_PIPE_NAME)).as_ptr(),
            FileSystem::PIPE_ACCESS_DUPLEX | FileSystem::FILE_FLAG_FIRST_PIPE_INSTANCE,
            Pipes::PIPE_TYPE_BYTE
                | Pipes::PIPE_READMODE_BYTE
                | Pipes::PIPE_WAIT
                | Pipes::PIPE_REJECT_REMOTE_CLIENTS,
            1,
            1024,
            (MAX_REQUEST_BYTES + size_of::<u32>()) as u32,
            0,
            &attributes,
        )
    };
    if pipe == Foundation::INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error()).context("create sandbox provisioning pipe");
    }
    let pipe = PipeHandle(pipe);
    on_ready()?;

    while !shutdown.load(Ordering::Acquire) {
        if !accept_connection(pipe.0)? {
            continue;
        }
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        let response = match handle_request(pipe.0, &shutdown) {
            Ok(()) => WindowsSandboxProvisioningResponse::Ok,
            Err(error) => WindowsSandboxProvisioningResponse::Error {
                message: bounded_error(&format!("{error:#}")),
            },
        };
        let frame = WindowsSandboxProvisioningFrame {
            version: SANDBOX_SERVICE_PROTOCOL_VERSION,
            message: WindowsSandboxProvisioningMessage::Result(response),
        };
        let mut bytes = Vec::new();
        write_provisioning_frame(&mut bytes, &frame)?;
        let mut written = 0;
        unsafe {
            FileSystem::WriteFile(
                pipe.0,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            );
            Pipes::DisconnectNamedPipe(pipe.0);
        }
    }
    Ok(())
}

pub(crate) fn wake(is_stopped: impl Fn() -> bool) {
    let pipe_name = to_wide(OsStr::new(SANDBOX_SERVICE_PIPE_NAME));
    while !is_stopped() {
        let handle = unsafe {
            FileSystem::CreateFileW(
                pipe_name.as_ptr(),
                Foundation::GENERIC_WRITE,
                0,
                ptr::null(),
                FileSystem::OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle != Foundation::INVALID_HANDLE_VALUE {
            unsafe { Foundation::CloseHandle(handle) };
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn accept_connection(pipe: HANDLE) -> Result<bool> {
    if unsafe { Pipes::ConnectNamedPipe(pipe, ptr::null_mut()) } != 0 {
        return Ok(true);
    }
    match unsafe { Foundation::GetLastError() } {
        Foundation::ERROR_PIPE_CONNECTED => Ok(true),
        Foundation::ERROR_NO_DATA | Foundation::ERROR_BROKEN_PIPE => {
            unsafe { Pipes::DisconnectNamedPipe(pipe) };
            Ok(false)
        }
        error => Err(std::io::Error::from_raw_os_error(error as i32))
            .context("accept sandbox provisioning client"),
    }
}

fn handle_request(pipe: HANDLE, shutdown: &AtomicBool) -> Result<()> {
    let process = authorize_client_process(pipe)?;
    let request = read_request(pipe, shutdown)?;
    if request.version != SANDBOX_SERVICE_PROTOCOL_VERSION {
        bail!(
            "unsupported sandbox service protocol version {}",
            request.version
        );
    }
    let WindowsSandboxProvisioningMessage::Provision(request) = request.message else {
        bail!("expected a sandbox provisioning request");
    };
    let authenticated = authenticate_request(pipe, &process, request)?;
    crate::worker::provision(&authenticated.request, shutdown)
}

fn read_request(pipe: HANDLE, shutdown: &AtomicBool) -> Result<WindowsSandboxProvisioningFrame> {
    let deadline = Instant::now() + REQUEST_IDLE_TIMEOUT;
    let mut request = [0_u8; MAX_REQUEST_BYTES + size_of::<u32>()];
    let mut request_length = 0;
    loop {
        if shutdown.load(Ordering::Acquire) {
            bail!("sandbox service is stopping");
        }
        if Instant::now() >= deadline {
            bail!("sandbox provisioning request timed out");
        }
        let mut available = 0;
        if unsafe {
            Pipes::PeekNamedPipe(
                pipe,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                &mut available,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error()).context("inspect sandbox request");
        }
        if available as usize > request.len() - request_length {
            bail!("sandbox provisioning request exceeds its size limit");
        }
        if available == 0 {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        let mut read = 0;
        if unsafe {
            FileSystem::ReadFile(
                pipe,
                request[request_length..].as_mut_ptr(),
                available,
                &mut read,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error()).context("read sandbox request");
        }
        if read == 0 {
            bail!("sandbox provisioning client sent an empty request");
        }
        request_length += read as usize;
        if request_length < size_of::<u32>() {
            continue;
        }
        let payload_length =
            u32::from_le_bytes(request[..size_of::<u32>()].try_into().unwrap()) as usize;
        if payload_length > MAX_REQUEST_BYTES {
            bail!("sandbox provisioning request exceeds its size limit");
        }
        let frame_length = size_of::<u32>() + payload_length;
        if request_length > frame_length {
            bail!("sandbox provisioning accepts exactly one request frame");
        }
        if request_length == frame_length {
            let mut bytes = &request[..request_length];
            return read_provisioning_frame(&mut bytes)?
                .context("sandbox provisioning client sent an empty frame");
        }
    }
}

fn bounded_error(error: &str) -> String {
    let mut message = String::new();
    for character in error.chars() {
        let character = if character.is_control() {
            ' '
        } else {
            character
        };
        if message.len() + character.len_utf8() > MAX_ERROR_BYTES {
            break;
        }
        message.push(character);
    }
    message
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
