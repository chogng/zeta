use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::ConnectionOptions;
use crate::endpoint::EndpointPaths;

const MAX_PID_RECORD_BYTES: u64 = 16 * 1024;
#[cfg(unix)]
const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(50);
#[cfg(unix)]
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const PROFILE_ROOT_ENV: &str = "ZETA_PROFILE_ROOT";
const WORKSPACE_ROOT_ENV: &str = "ZETA_WORKSPACE_ROOT";
const WORKSPACE_TRUST_SOURCE_ENV: &str = "ZETA_WORKSPACE_TRUST_SOURCE";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessRecord {
    pub(crate) pid: u32,
    pub(crate) instance_id: String,
    pub(crate) process_start_identity: Option<String>,
    pub(crate) daemon_version: String,
}

impl ProcessRecord {
    pub(crate) fn current(endpoint: &EndpointPaths) -> Result<Self, String> {
        let pid = std::process::id();
        let process_start_identity = process_start_identity(pid)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        let mut digest = Sha256::new();
        digest.update(endpoint.socket.to_string_lossy().as_bytes());
        digest.update(pid.to_le_bytes());
        digest.update(now.to_le_bytes());
        if let Some(identity) = &process_start_identity {
            digest.update(identity.as_bytes());
        }
        Ok(Self {
            pid,
            instance_id: format!("{:x}", digest.finalize()),
            process_start_identity,
            daemon_version: env!("CARGO_PKG_VERSION").into(),
        })
    }
}

pub(crate) struct ProcessRecordGuard {
    path: PathBuf,
    instance_id: String,
}

impl ProcessRecordGuard {
    pub(crate) fn publish(path: &Path, record: &ProcessRecord) -> Result<Self, String> {
        let temp = path.with_extension(format!("{}.tmp", record.instance_id));
        let contents = serde_json::to_vec(record).map_err(|error| error.to_string())?;
        let mut file = open_private_record(&temp)?;
        file.write_all(&contents).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        if cfg!(windows) && path.exists() {
            fs::remove_file(path).map_err(io_error)?;
        }
        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(error));
        }
        Ok(Self {
            path: path.to_path_buf(),
            instance_id: record.instance_id.clone(),
        })
    }
}

impl Drop for ProcessRecordGuard {
    fn drop(&mut self) {
        if read_process_record(&self.path)
            .ok()
            .flatten()
            .is_some_and(|record| record.instance_id == self.instance_id)
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) fn read_process_record(path: &Path) -> Result<Option<ProcessRecord>, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(error)),
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_PID_RECORD_BYTES
    {
        return Err("Local App Server process record is not a bounded regular file".into());
    }
    let contents = fs::read(path).map_err(io_error)?;
    serde_json::from_slice(&contents)
        .map(Some)
        .map_err(|error| format!("invalid Local App Server process record: {error}"))
}

pub(crate) fn remove_stale_process_record(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

pub(crate) fn spawn_daemon(
    endpoint: &EndpointPaths,
    options: &ConnectionOptions,
    daemon_executable: &Path,
) -> Result<u32, String> {
    let metadata = fs::symlink_metadata(daemon_executable).map_err(io_error)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "Local App Server daemon is not a regular executable: {}",
            daemon_executable.display()
        ));
    }
    let log = endpoint.open_log()?;
    let error_log = log.try_clone().map_err(io_error)?;
    let mut command = Command::new(daemon_executable);
    command
        .env(PROFILE_ROOT_ENV, options.profile_root())
        .env_remove(WORKSPACE_ROOT_ENV)
        .env_remove(WORKSPACE_TRUST_SOURCE_ENV)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log));
    detach_command(&mut command);
    command.spawn().map(|child| child.id()).map_err(io_error)
}

#[cfg(unix)]
fn detach_command(command: &mut Command) {
    command.process_group(0);
}

#[cfg(windows)]
fn detach_command(command: &mut Command) {
    use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

#[cfg(unix)]
pub(crate) fn force_terminate(record: &ProcessRecord) -> Result<(), String> {
    let Some(expected_start) = &record.process_start_identity else {
        return Err("managed daemon has no process start identity".into());
    };
    if process_start_identity(record.pid)?.as_ref() != Some(expected_start) {
        return Err("refusing to terminate a reused or stale daemon pid".into());
    }
    let status = Command::new("kill")
        .arg("-KILL")
        .arg(record.pid.to_string())
        .status()
        .map_err(io_error)?;
    if !status.success() {
        return Err(format!(
            "failed to force terminate Local App Server daemon {}",
            record.pid
        ));
    }
    wait_for_process_exit(record)
}

#[cfg(windows)]
pub(crate) fn force_terminate(_record: &ProcessRecord) -> Result<(), String> {
    Err("managed daemon did not honor its authenticated stop request; automatic force termination is unavailable on Windows".into())
}

#[cfg(unix)]
fn wait_for_process_exit(record: &ProcessRecord) -> Result<(), String> {
    let deadline = std::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while std::time::Instant::now() < deadline {
        if process_start_identity(record.pid)?.as_ref() != record.process_start_identity.as_ref() {
            return Ok(());
        }
        thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
    Err(format!(
        "timed out waiting for Local App Server daemon {} to exit",
        record.pid
    ))
}

#[cfg(unix)]
fn process_start_identity(pid: u32) -> Result<Option<String>, String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "lstart="])
        .output()
        .map_err(io_error)?;
    if !output.status.success() {
        return Ok(None);
    }
    let identity = String::from_utf8(output.stdout)
        .map_err(|error| error.to_string())?
        .trim()
        .to_string();
    Ok((!identity.is_empty()).then_some(identity))
}

#[cfg(windows)]
fn process_start_identity(_pid: u32) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(unix)]
fn open_private_record(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(io_error)
}

#[cfg(windows)]
fn open_private_record(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(io_error)
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
