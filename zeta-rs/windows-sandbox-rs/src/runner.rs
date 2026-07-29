use crate::appcontainer::{
    AppContainerSid, OwnedHandle, canonical_directory, canonical_file, command_line, last_error,
    to_wide,
};
use crate::profile_name;
use crate::protocol::{
    ACCESS_FLAG, COMMAND_SEPARATOR, CWD_FLAG, ENFORCEMENT_FAILURE_EXIT_CODE, ERROR_PREFIX,
    PROBE_FLAG, PROGRAM_FLAG, READ_ONLY_ACCESS, RUNNER_PROBE, SETUP_HELPER_FLAG, WORKSPACE_FLAG,
    WORKSPACE_WRITE_ACCESS, remap_inner_exit_code,
};
use std::ffi::{OsStr, OsString, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::{
    HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation, WAIT_FAILED,
};
use windows_sys::Win32::Security::SECURITY_CAPABILITIES;
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE,
    InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute,
    WaitForSingleObject,
};
use windows_sys::Win32::System::WindowsProgramming::PROCESS_CREATION_CHILD_PROCESS_RESTRICTED;

pub(crate) fn main() -> ! {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(exit_code) => std::process::exit(remap_inner_exit_code(exit_code)),
        Err(error) => {
            eprintln!("{ERROR_PREFIX} {error}");
            std::process::exit(ENFORCEMENT_FAILURE_EXIT_CODE)
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<i32, String> {
    if arguments.as_slice() == [OsString::from(PROBE_FLAG)] {
        println!("{RUNNER_PROBE}");
        return Ok(0);
    }
    let request = RunnerRequest::parse(arguments)?;
    let setup_helper = canonical_file(&request.setup_helper, "sandbox setup helper")?;
    let workspace = canonical_directory(&request.workspace, "workspace")?;
    let cwd = canonical_directory(&request.cwd, "working directory")?;
    if !cwd.starts_with(&workspace) {
        return Err("working directory resolves outside workspace".to_owned());
    }
    let access = request
        .access
        .to_str()
        .ok_or("filesystem access mode is not valid Unicode")?;
    if !matches!(access, READ_ONLY_ACCESS | WORKSPACE_WRITE_ACCESS) {
        return Err("unsupported filesystem access mode".to_owned());
    }
    let profile = profile_name(&workspace, access);
    let source_program = canonical_file(
        PathBuf::from(&request.command[0]).as_path(),
        "sandboxed program",
    )?;
    let staged_program = StagedProgram::copy_from(&source_program)?;
    let program = staged_program.path();

    let setup_output = Command::new(setup_helper)
        .arg(ACCESS_FLAG)
        .arg(&request.access)
        .arg(WORKSPACE_FLAG)
        .arg(&workspace)
        .arg(PROGRAM_FLAG)
        .arg(program)
        .output()
        .map_err(|error| format!("could not run sandbox setup helper: {error}"))?;
    if !setup_output.status.success() {
        return Err(format!(
            "sandbox setup helper failed: {}",
            String::from_utf8_lossy(&setup_output.stderr).trim()
        ));
    }

    let sid = AppContainerSid::ensure(OsStr::new(&profile))?;
    let mut command = request.command;
    command[0] = program.as_os_str().to_owned();
    launch(&sid, &command, &cwd)
}

static NEXT_STAGED_PROGRAM: AtomicU64 = AtomicU64::new(0);

struct StagedProgram {
    directory: PathBuf,
    path: PathBuf,
}

impl StagedProgram {
    fn copy_from(source: &std::path::Path) -> Result<Self, String> {
        let temporary_root = std::env::temp_dir();
        for _ in 0..128 {
            let sequence = NEXT_STAGED_PROGRAM.fetch_add(1, Ordering::Relaxed);
            let directory = temporary_root.join(format!(
                "zeta-sandbox-program-{}-{sequence}",
                std::process::id()
            ));
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    let path = directory.join("program.exe");
                    if let Err(error) = std::fs::copy(source, &path) {
                        let _ = std::fs::remove_dir(&directory);
                        return Err(format!("could not stage sandboxed program: {error}"));
                    }
                    let path = match canonical_file(&path, "staged sandboxed program") {
                        Ok(path) => path,
                        Err(error) => {
                            let _ = std::fs::remove_file(&path);
                            let _ = std::fs::remove_dir(&directory);
                            return Err(error);
                        }
                    };
                    return Ok(Self { directory, path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "could not create sandboxed program directory: {error}"
                    ));
                }
            }
        }
        Err("could not allocate a unique sandboxed program directory".to_owned())
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for StagedProgram {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

struct RunnerRequest {
    setup_helper: PathBuf,
    access: OsString,
    workspace: PathBuf,
    cwd: PathBuf,
    command: Vec<OsString>,
}

impl RunnerRequest {
    fn parse(arguments: Vec<OsString>) -> Result<Self, String> {
        let mut setup_helper = None;
        let mut access = None;
        let mut workspace = None;
        let mut cwd = None;
        let mut command = None;
        let mut arguments = arguments.into_iter();
        while let Some(flag) = arguments.next() {
            if flag == COMMAND_SEPARATOR {
                command = Some(arguments.collect());
                break;
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))?;
            match flag.to_str() {
                Some(SETUP_HELPER_FLAG) => setup_helper = Some(PathBuf::from(value)),
                Some(ACCESS_FLAG) => access = Some(value),
                Some(WORKSPACE_FLAG) => workspace = Some(PathBuf::from(value)),
                Some(CWD_FLAG) => cwd = Some(PathBuf::from(value)),
                _ => return Err(format!("unexpected argument {}", flag.to_string_lossy())),
            }
        }
        let command: Vec<OsString> = command.ok_or("missing command separator")?;
        if command.is_empty() {
            return Err("missing sandboxed command".to_owned());
        }
        Ok(Self {
            setup_helper: setup_helper.ok_or("missing setup helper")?,
            access: access.ok_or("missing access mode")?,
            workspace: workspace.ok_or("missing workspace")?,
            cwd: cwd.ok_or("missing working directory")?,
            command,
        })
    }
}

fn launch(
    sid: &AppContainerSid,
    arguments: &[OsString],
    cwd: &std::path::Path,
) -> Result<i32, String> {
    let job = create_job()?;
    let mut attributes = AttributeList::new(2)?;
    let mut security_capabilities = SECURITY_CAPABILITIES {
        AppContainerSid: sid.as_ptr(),
        Capabilities: std::ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };
    attributes.set(
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
        &mut security_capabilities as *mut _ as *const c_void,
        std::mem::size_of::<SECURITY_CAPABILITIES>(),
    )?;
    let child_policy = PROCESS_CREATION_CHILD_PROCESS_RESTRICTED;
    attributes.set(
        PROC_THREAD_ATTRIBUTE_CHILD_PROCESS_POLICY as usize,
        &child_policy as *const _ as *const c_void,
        std::mem::size_of_val(&child_policy),
    )?;

    let stdin = inheritable_standard_handle(STD_INPUT_HANDLE)?;
    let stdout = inheritable_standard_handle(STD_OUTPUT_HANDLE)?;
    let stderr = inheritable_standard_handle(STD_ERROR_HANDLE)?;
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin;
    startup.StartupInfo.hStdOutput = stdout;
    startup.StartupInfo.hStdError = stderr;
    startup.lpAttributeList = attributes.pointer();

    let mut command_line = command_line(arguments);
    let cwd = to_wide(cwd.as_os_str());
    let environment = minimal_environment();
    let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        CreateProcessW(
            std::ptr::null(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW,
            environment.as_ptr() as *const c_void,
            cwd.as_ptr(),
            &startup.StartupInfo,
            &mut process,
        )
    };
    if created == 0 {
        return Err(last_error("CreateProcessW AppContainer launch"));
    }
    let process_handle = OwnedHandle::new(process.hProcess, "child process handle")?;
    let thread_handle = OwnedHandle::new(process.hThread, "child thread handle")?;
    if unsafe { AssignProcessToJobObject(job.raw(), process_handle.raw()) } == 0 {
        unsafe {
            TerminateProcess(process_handle.raw(), 1);
        }
        return Err(last_error("AssignProcessToJobObject"));
    }
    if unsafe { ResumeThread(thread_handle.raw()) } == u32::MAX {
        unsafe {
            TerminateProcess(process_handle.raw(), 1);
        }
        return Err(last_error("ResumeThread"));
    }
    if unsafe { WaitForSingleObject(process_handle.raw(), INFINITE) } == WAIT_FAILED {
        return Err(last_error("WaitForSingleObject"));
    }
    let mut exit_code = 1;
    if unsafe { GetExitCodeProcess(process_handle.raw(), &mut exit_code) } == 0 {
        return Err(last_error("GetExitCodeProcess"));
    }
    Ok(exit_code as i32)
}

fn create_job() -> Result<OwnedHandle, String> {
    let job = OwnedHandle::new(
        unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) },
        "CreateJobObjectW",
    )?;
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    limits.BasicLimitInformation.ActiveProcessLimit = 1;
    if unsafe {
        SetInformationJobObject(
            job.raw(),
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const c_void,
            std::mem::size_of_val(&limits) as u32,
        )
    } == 0
    {
        return Err(last_error("SetInformationJobObject"));
    }
    Ok(job)
}

fn inheritable_standard_handle(kind: u32) -> Result<HANDLE, String> {
    let handle = unsafe { GetStdHandle(kind) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(last_error("GetStdHandle"));
    }
    if unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) } == 0 {
        return Err(last_error("SetHandleInformation"));
    }
    Ok(handle)
}

fn minimal_environment() -> Vec<u16> {
    let mut values = Vec::new();
    for name in ["SystemRoot", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            let entry = OsString::from(format!("{name}={}", value.to_string_lossy()));
            values.extend(to_wide(&entry)[..].iter().copied());
        }
    }
    values.extend(to_wide(OsStr::new("ZETA_SANDBOX=appcontainer")));
    values.push(0);
    values
}

struct AttributeList {
    storage: Vec<usize>,
    pointer: *mut c_void,
}

impl AttributeList {
    fn new(count: u32) -> Result<Self, String> {
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(std::ptr::null_mut(), count, 0, &mut bytes);
        }
        if bytes == 0 {
            return Err(last_error("InitializeProcThreadAttributeList sizing"));
        }
        let words = bytes.div_ceil(std::mem::size_of::<usize>());
        let mut storage = vec![0usize; words];
        let pointer = storage.as_mut_ptr() as *mut c_void;
        if unsafe { InitializeProcThreadAttributeList(pointer, count, 0, &mut bytes) } == 0 {
            return Err(last_error("InitializeProcThreadAttributeList"));
        }
        Ok(Self { storage, pointer })
    }

    fn set(&mut self, attribute: usize, value: *const c_void, size: usize) -> Result<(), String> {
        if unsafe {
            UpdateProcThreadAttribute(
                self.pointer,
                0,
                attribute,
                value,
                size,
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        } == 0
        {
            return Err(last_error("UpdateProcThreadAttribute"));
        }
        Ok(())
    }

    fn pointer(&mut self) -> *mut c_void {
        self.storage.as_mut_ptr() as *mut c_void
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.pointer);
        }
    }
}
