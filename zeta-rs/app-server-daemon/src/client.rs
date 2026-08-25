use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use serde_json::json;
use zeta_app_server_protocol::schema_hash;
use zeta_uds::UnixStream;

use crate::ConnectionOptions;
use crate::LifecycleCommand;
use crate::LifecycleOutput;
use crate::LifecycleStatus;
use crate::endpoint::EndpointPaths;
use crate::endpoint::connect_existing;
use crate::process::ProcessRecord;
use crate::process::executable_identity;
use crate::process::force_terminate;
use crate::process::read_process_record;
use crate::process::remove_stale_process_record;
use crate::process::spawn_daemon;
use crate::wire::ConnectionPrelude;
use crate::wire::ControlCommand;
use crate::wire::ControlPrelude;
use crate::wire::ControlResponse;
use crate::wire::ControlState;
use crate::wire::write_json_line;

const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONTROL_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const START_TIMEOUT: Duration = Duration::from_secs(15);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeInfo {
    server_name: String,
    schema_hash: String,
}

pub(crate) fn run_lifecycle(
    command: LifecycleCommand,
    options: ConnectionOptions,
    daemon_executable: &Path,
) -> Result<LifecycleOutput, String> {
    let endpoint = EndpointPaths::prepare(options.profile_root())?;
    let _operation_lock = endpoint.acquire_operation_lock()?;
    match command {
        LifecycleCommand::Start => start_unlocked(&endpoint, &options, daemon_executable),
        LifecycleCommand::Restart => {
            let _ = stop_unlocked(&endpoint)?;
            let mut output = start_unlocked(&endpoint, &options, daemon_executable)?;
            output.status = LifecycleStatus::Restarted;
            Ok(output)
        }
        LifecycleCommand::Stop => stop_unlocked(&endpoint),
        LifecycleCommand::Version => version_unlocked(&endpoint, &options, daemon_executable),
    }
}

pub(crate) fn connect(options: ConnectionOptions, daemon_executable: &Path) -> Result<(), String> {
    run_lifecycle(LifecycleCommand::Start, options.clone(), daemon_executable)?;
    let endpoint = EndpointPaths::prepare(options.profile_root())?;
    let stream = connect_existing(&endpoint.socket)?
        .ok_or_else(|| "Local App Server daemon exited before the client connected".to_string())?;
    proxy_stdio(stream, &options).map_err(|error| error.to_string())
}

fn start_unlocked(
    endpoint: &EndpointPaths,
    options: &ConnectionOptions,
    daemon_executable: &Path,
) -> Result<LifecycleOutput, String> {
    let mut replaced_stale_daemon = false;
    if let Some(control) = request_control(endpoint, ControlCommand::Status)? {
        if control.state == ControlState::Stopping {
            return Err("Local App Server daemon is stopping".into());
        }
        let record = validate_managed_response(endpoint, &control)?;
        if validate_executable_identity(&record, daemon_executable).is_ok() {
            let probe = probe_app_server(endpoint, options)
                .map_err(|error| diagnostic_error(endpoint, &error))?;
            return Ok(lifecycle_output(
                LifecycleStatus::AlreadyRunning,
                endpoint,
                Some(&control),
                Some(&probe),
            ));
        }
        let _ = stop_unlocked(endpoint)?;
        replaced_stale_daemon = true;
    }

    remove_stale_process_record(&endpoint.pid)?;
    let _spawned_pid = spawn_daemon(endpoint, options, daemon_executable)?;
    let deadline = Instant::now() + START_TIMEOUT;
    let mut last_error = None;
    while Instant::now() < deadline {
        match request_control(endpoint, ControlCommand::Status) {
            Ok(Some(control)) if control.state == ControlState::Running => {
                let record = validate_managed_response(endpoint, &control)
                    .map_err(|error| diagnostic_error(endpoint, &error))?;
                validate_executable_identity(&record, daemon_executable)
                    .map_err(|error| diagnostic_error(endpoint, &error))?;
                let probe = probe_app_server(endpoint, options)
                    .map_err(|error| diagnostic_error(endpoint, &error))?;
                return Ok(lifecycle_output(
                    if replaced_stale_daemon {
                        LifecycleStatus::Restarted
                    } else {
                        LifecycleStatus::Started
                    },
                    endpoint,
                    Some(&control),
                    Some(&probe),
                ));
            }
            Ok(Some(_)) | Ok(None) => {}
            Err(error) => last_error = Some(error),
        }
        thread::sleep(CONNECT_RETRY_INTERVAL);
    }
    let reason = last_error.unwrap_or_else(|| "daemon control endpoint was unavailable".into());
    Err(diagnostic_error(
        endpoint,
        &format!("Local App Server daemon did not become ready: {reason}"),
    ))
}

fn stop_unlocked(endpoint: &EndpointPaths) -> Result<LifecycleOutput, String> {
    let Some(control) = request_control(endpoint, ControlCommand::Status)? else {
        remove_stale_process_record(&endpoint.pid)?;
        return Ok(lifecycle_output(
            LifecycleStatus::NotRunning,
            endpoint,
            None,
            None,
        ));
    };
    let record = validate_managed_response(endpoint, &control)?;
    let stop = request_control(endpoint, ControlCommand::Stop)?
        .ok_or_else(|| "Local App Server daemon exited before acknowledging stop".to_string())?;
    if stop.instance_id != control.instance_id || stop.pid != control.pid {
        return Err("Local App Server daemon changed generation during stop".into());
    }

    let deadline = Instant::now() + STOP_TIMEOUT;
    while Instant::now() < deadline {
        if connect_existing(&endpoint.socket)?.is_none() {
            remove_stale_process_record(&endpoint.pid)?;
            return Ok(lifecycle_output(
                LifecycleStatus::Stopped,
                endpoint,
                Some(&control),
                None,
            ));
        }
        thread::sleep(CONNECT_RETRY_INTERVAL);
    }

    force_terminate(&record).map_err(|error| diagnostic_error(endpoint, &error))?;
    remove_stale_process_record(&endpoint.pid)?;
    Ok(lifecycle_output(
        LifecycleStatus::Stopped,
        endpoint,
        Some(&control),
        None,
    ))
}

fn version_unlocked(
    endpoint: &EndpointPaths,
    options: &ConnectionOptions,
    daemon_executable: &Path,
) -> Result<LifecycleOutput, String> {
    let Some(control) = request_control(endpoint, ControlCommand::Status)? else {
        remove_stale_process_record(&endpoint.pid)?;
        return Ok(lifecycle_output(
            LifecycleStatus::NotRunning,
            endpoint,
            None,
            None,
        ));
    };
    if control.state == ControlState::Stopping {
        return Err("Local App Server daemon is stopping".into());
    }
    let record = validate_managed_response(endpoint, &control)?;
    validate_executable_identity(&record, daemon_executable)?;
    let probe =
        probe_app_server(endpoint, options).map_err(|error| diagnostic_error(endpoint, &error))?;
    Ok(lifecycle_output(
        LifecycleStatus::Running,
        endpoint,
        Some(&control),
        Some(&probe),
    ))
}

fn validate_executable_identity(
    record: &ProcessRecord,
    daemon_executable: &Path,
) -> Result<(), String> {
    let expected = executable_identity(daemon_executable)?;
    if record.executable_identity.as_ref() != Some(&expected) {
        return Err(format!(
            "running Local App Server daemon executable is stale: {}",
            daemon_executable.display()
        ));
    }
    Ok(())
}

fn lifecycle_output(
    status: LifecycleStatus,
    endpoint: &EndpointPaths,
    control: Option<&ControlResponse>,
    probe: Option<&ProbeInfo>,
) -> LifecycleOutput {
    LifecycleOutput {
        status,
        pid: control.map(|control| control.pid),
        instance_id: control.map(|control| control.instance_id.clone()),
        daemon_version: control
            .map(|control| control.daemon_version.clone())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").into()),
        endpoint_path: endpoint.socket.clone(),
        log_path: endpoint.log.clone(),
        app_server_name: probe.map(|probe| probe.server_name.clone()),
        schema_hash: probe.map(|probe| probe.schema_hash.clone()),
    }
}

fn validate_managed_response(
    endpoint: &EndpointPaths,
    control: &ControlResponse,
) -> Result<ProcessRecord, String> {
    control.validate()?;
    if control.daemon_version != env!("CARGO_PKG_VERSION") || control.schema_hash != schema_hash() {
        return Err("running Local App Server daemon is incompatible with this client".into());
    }
    let record = read_process_record(&endpoint.pid)?.ok_or_else(|| {
        "App Server daemon endpoint is running without a managed process record".to_string()
    })?;
    if record.pid != control.pid
        || record.instance_id != control.instance_id
        || record.daemon_version != control.daemon_version
    {
        return Err("App Server daemon endpoint does not match its managed process record".into());
    }
    Ok(record)
}

fn request_control(
    endpoint: &EndpointPaths,
    command: ControlCommand,
) -> Result<Option<ControlResponse>, String> {
    let Some(mut stream) = connect_existing(&endpoint.socket)? else {
        return Ok(None);
    };
    stream
        .set_read_timeout(Some(CONTROL_TIMEOUT))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(CONTROL_TIMEOUT))
        .map_err(io_error)?;
    write_json_line(&mut stream, &ControlPrelude::new(command)).map_err(io_error)?;
    let mut line = String::new();
    let read = BufReader::new(stream)
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_line(&mut line)
        .map_err(io_error)?;
    if read == 0 || read > MAX_RESPONSE_BYTES || !line.ends_with('\n') {
        return Err("Local App Server daemon returned an invalid control response".into());
    }
    let response: ControlResponse =
        serde_json::from_str(&line).map_err(|error| error.to_string())?;
    response.validate()?;
    Ok(Some(response))
}

fn probe_app_server(
    endpoint: &EndpointPaths,
    options: &ConnectionOptions,
) -> Result<ProbeInfo, String> {
    let mut stream = connect_existing(&endpoint.socket)?
        .ok_or_else(|| "Local App Server daemon control endpoint is unavailable".to_string())?;
    stream
        .set_read_timeout(Some(PROBE_TIMEOUT))
        .map_err(io_error)?;
    stream
        .set_write_timeout(Some(PROBE_TIMEOUT))
        .map_err(io_error)?;
    write_json_line(&mut stream, &ConnectionPrelude::from_options(options)).map_err(io_error)?;
    write_json_line(
        &mut stream,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "zeta-app-server-daemon-probe",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {},
            },
        }),
    )
    .map_err(io_error)?;

    let mut line = String::new();
    let read = BufReader::new(stream)
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_line(&mut line)
        .map_err(io_error)?;
    if read == 0 || read > MAX_RESPONSE_BYTES || !line.ends_with('\n') {
        return Err("App Server initialize probe returned no bounded response".into());
    }
    let response: Value = serde_json::from_str(&line).map_err(|error| error.to_string())?;
    if let Some(error) = response.get("error") {
        return Err(format!("App Server initialize probe failed: {error}"));
    }
    if response.get("id") != Some(&Value::from(1)) {
        return Err("App Server initialize probe returned the wrong request id".into());
    }
    let server_name = response
        .pointer("/result/serverInfo/name")
        .and_then(Value::as_str)
        .ok_or_else(|| "App Server initialize response has no server name".to_string())?
        .to_string();
    let initialized_schema = response
        .pointer("/result/schemaHash")
        .and_then(Value::as_str)
        .ok_or_else(|| "App Server initialize response has no schema hash".to_string())?
        .to_string();
    if server_name != "zeta-app-server" || initialized_schema != schema_hash() {
        return Err(format!(
            "App Server initialize contract mismatch: server={server_name}, schema={initialized_schema}"
        ));
    }
    Ok(ProbeInfo {
        server_name,
        schema_hash: initialized_schema,
    })
}

fn proxy_stdio(mut stream: UnixStream, options: &ConnectionOptions) -> io::Result<()> {
    write_json_line(&mut stream, &ConnectionPrelude::from_options(options))?;
    let mut socket_writer = stream.try_clone()?;
    let input = thread::Builder::new()
        .name("zeta-local-app-server-stdin".into())
        .spawn(move || {
            let copied = io::copy(&mut io::stdin().lock(), &mut socket_writer);
            let _ = socket_writer.shutdown(Shutdown::Write);
            copied
        })?;
    let mut output = io::stdout().lock();
    io::copy(&mut BufReader::new(stream), &mut output)?;
    output.flush()?;
    input
        .join()
        .map_err(|_| io::Error::other("Local App Server stdin proxy panicked"))??;
    Ok(())
}

fn diagnostic_error(endpoint: &EndpointPaths, message: &str) -> String {
    match endpoint.log_tail() {
        Some(log) => format!(
            "{message}\n\nManaged daemon log ({}):\n{log}",
            endpoint.log.display()
        ),
        None => format!("{message}; inspect {}", endpoint.log.display()),
    }
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
