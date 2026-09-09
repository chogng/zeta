//! Runs ACL work in a one-process Job Object so malformed trees cannot take down the service.

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::process::Stdio;
use std::ptr;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Foundation::NTSTATUS;
use windows_sys::Win32::System::JobObjects;
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;
use zeta_windows_sandbox::SANDBOX_WORKER_EXECUTABLE_NAME;
use zeta_windows_sandbox::WindowsSandboxProvisioningRequest;

const WORKER_TIMEOUT: Duration = Duration::from_secs(120);
const WORKER_MEMORY_LIMIT_BYTES: usize = 512 * 1024 * 1024;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtResumeProcess(process_handle: HANDLE) -> NTSTATUS;
}

pub(crate) fn provision(
    request: &WindowsSandboxProvisioningRequest,
    shutdown: &AtomicBool,
) -> Result<()> {
    let worker = std::env::current_exe()
        .context("locate the sandbox service executable")?
        .with_file_name(SANDBOX_WORKER_EXECUTABLE_NAME);
    let metadata = worker
        .symlink_metadata()
        .with_context(|| format!("inspect provisioning worker {}", worker.display()))?;
    if !metadata.is_file()
        || metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    {
        bail!("refusing invalid provisioning worker {}", worker.display());
    }
    let request =
        serde_json::to_string(request).context("serialize provisioning worker request")?;
    let job = WorkerJob::new()?;
    let mut command = Command::new(&worker);
    command
        .arg(request)
        .creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .with_context(|| format!("start provisioning worker {}", worker.display()))?;
    let process = child.as_raw_handle() as HANDLE;
    if unsafe { JobObjects::AssignProcessToJobObject(job.0, process) } == 0 {
        let error = std::io::Error::last_os_error();
        let _ = child.kill();
        let _ = child.wait();
        return Err(error).context("assign provisioning worker to its Job Object");
    }
    let status = unsafe { NtResumeProcess(process) };
    if status < 0 {
        let _ = child.kill();
        let _ = child.wait();
        bail!("resume provisioning worker failed with NTSTATUS {status:#x}");
    }

    let deadline = Instant::now() + WORKER_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().context("wait for provisioning worker")? {
            if status.success() {
                return Ok(());
            }
            bail!("provisioning worker exited with {status}");
        }
        if shutdown.load(Ordering::Acquire) {
            job.terminate()?;
            let _ = child.wait();
            bail!("provisioning worker stopped with the sandbox service");
        }
        if Instant::now() >= deadline {
            job.terminate()?;
            let _ = child.wait();
            bail!("provisioning worker exceeded its 120 second limit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

struct WorkerJob(HANDLE);

impl WorkerJob {
    fn new() -> Result<Self> {
        let handle = unsafe { JobObjects::CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error()).context("create provisioning Job Object");
        }
        let job = Self(handle);
        let mut limits: JobObjects::JOBOBJECT_EXTENDED_LIMIT_INFORMATION =
            unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JobObjects::JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JobObjects::JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JobObjects::JOB_OBJECT_LIMIT_JOB_MEMORY;
        limits.BasicLimitInformation.ActiveProcessLimit = 1;
        limits.JobMemoryLimit = WORKER_MEMORY_LIMIT_BYTES;
        if unsafe {
            JobObjects::SetInformationJobObject(
                handle,
                JobObjects::JobObjectExtendedLimitInformation,
                (&raw mut limits).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error())
                .context("limit the provisioning Job Object");
        }
        Ok(job)
    }

    fn terminate(&self) -> Result<()> {
        if unsafe { JobObjects::TerminateJobObject(self.0, 1) } == 0 {
            return Err(std::io::Error::last_os_error())
                .context("terminate the provisioning Job Object");
        }
        Ok(())
    }
}

impl Drop for WorkerJob {
    fn drop(&mut self) {
        unsafe { Foundation::CloseHandle(self.0) };
    }
}
