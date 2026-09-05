use std::io;
use std::io::BufReader;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use zeta_app_server_protocol::schema_hash;
use zeta_app_server_transport::DeadlineStream;
use zeta_app_server_transport::LocalConnections;
use zeta_app_server_transport::LocalSocketAccept;
use zeta_app_server_transport::PollingLocalListener;

use crate::ConnectionOptions;
use crate::endpoint::EndpointPaths;
use crate::endpoint::SocketCleanup;
use crate::process::ProcessRecord;
use crate::process::ProcessRecordGuard;
use crate::registry::ProfileAppServerRegistry;
use crate::wire::CONNECTION_PRELUDE_TIMEOUT;
use crate::wire::ControlCommand;
use crate::wire::ControlResponse;
use crate::wire::ControlState;
use crate::wire::IncomingPrelude;
use crate::wire::read_prelude;
use crate::wire::write_json_line;

const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const IDLE_TIMEOUT_ENV: &str = "ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS";
const LOG_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(1);
const STOP_GRACE_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

pub(crate) fn serve(options: ConnectionOptions) -> Result<(), String> {
    let endpoint = EndpointPaths::prepare(options.profile_root())?;
    let registry = Arc::new(ProfileAppServerRegistry::open(options)?);
    let listener = PollingLocalListener::new(endpoint.bind_listener()?).map_err(io_error)?;
    let socket_cleanup = SocketCleanup::new(endpoint.socket.clone());
    let record = ProcessRecord::current(&endpoint)?;
    let process_record = ProcessRecordGuard::publish(&endpoint.pid, &record)?;
    let stop_requested = Arc::new(AtomicBool::new(false));
    register_shutdown_signals(&stop_requested)?;
    let mut automation = Some(registry.start_automation()?);
    eprintln!(
        "local App Server daemon endpoint ready: {} (pid {})",
        endpoint.socket.display(),
        record.pid
    );

    let active_connections = Arc::new(LocalConnections::new());
    let idle_timeout = configured_idle_timeout()?;
    let mut idle_since = None;
    let mut stopping_since = None;
    let mut connection_shutdown_since = None;
    let mut last_log_maintenance = Instant::now();
    loop {
        if last_log_maintenance.elapsed() >= LOG_MAINTENANCE_INTERVAL {
            let _ = endpoint.open_log();
            last_log_maintenance = Instant::now();
        }
        if stop_requested.load(Ordering::Acquire) {
            drop(automation.take());
            let stopping_since = stopping_since.get_or_insert_with(Instant::now);
            let active_connection_count = active_connections.len();
            let active_terminal_count = registry.active_terminal_count();
            if active_connection_count == 0 && active_terminal_count == 0 {
                exit_after_stop(listener, socket_cleanup, process_record);
            }
            if stopping_since.elapsed() >= STOP_GRACE_TIMEOUT {
                let shutdown_since = connection_shutdown_since.get_or_insert_with(|| {
                    active_connections.shutdown_all();
                    Instant::now()
                });
                if active_connections.is_empty()
                    || shutdown_since.elapsed() >= STOP_CONNECTION_DRAIN_TIMEOUT
                {
                    exit_after_stop(listener, socket_cleanup, process_record);
                }
            }
            thread::sleep(IDLE_POLL_INTERVAL);
            continue;
        }

        match listener.poll_accept() {
            Ok(LocalSocketAccept::Accepted(stream)) => {
                idle_since = None;
                let reader = match stream.try_clone() {
                    Ok(reader) => reader,
                    Err(error) => {
                        eprintln!("local App Server connection clone failed: {error}");
                        continue;
                    }
                };
                let deadline = Instant::now() + CONNECTION_PRELUDE_TIMEOUT;
                let mut stream = match DeadlineStream::new(stream, deadline) {
                    Ok(stream) => stream,
                    Err(error) => {
                        eprintln!("local App Server connection write deadline failed: {error}");
                        continue;
                    }
                };
                let reader = match DeadlineStream::new(reader, deadline) {
                    Ok(reader) => reader,
                    Err(error) => {
                        eprintln!("local App Server connection read deadline failed: {error}");
                        continue;
                    }
                };
                let mut reader = BufReader::new(reader);
                let prelude = match read_prelude(&mut reader) {
                    Ok(prelude) => prelude,
                    Err(error) => {
                        eprintln!("local App Server connection prelude failed: {error}");
                        continue;
                    }
                };
                match prelude {
                    IncomingPrelude::Control(control) => {
                        let stopping = matches!(control.command, ControlCommand::Stop);
                        if stopping {
                            stop_requested.store(true, Ordering::Release);
                        }
                        let response = ControlResponse::new(
                            if stopping {
                                ControlState::Stopping
                            } else {
                                ControlState::Running
                            },
                            record.pid,
                            record.instance_id.clone(),
                            schema_hash(),
                        );
                        let mut stream = stream;
                        if let Err(error) = write_json_line(&mut stream, &response)
                            && !is_peer_disconnect(&error)
                        {
                            eprintln!("local App Server control response failed: {error}");
                        }
                    }
                    IncomingPrelude::Connection(connection) => {
                        if let Err(error) = reader.get_mut().clear_deadline() {
                            eprintln!(
                                "local App Server connection read deadline cleanup failed: {error}"
                            );
                            continue;
                        }
                        if let Err(error) = stream.clear_deadline() {
                            eprintln!(
                                "local App Server connection write deadline cleanup failed: {error}"
                            );
                            continue;
                        }
                        let server = match registry.server_for(connection) {
                            Ok(server) => server,
                            Err(error) => {
                                eprintln!("local App Server directory runtime failed: {error}");
                                continue;
                            }
                        };
                        let shutdown_stream = match stream.try_clone() {
                            Ok(stream) => stream,
                            Err(error) => {
                                eprintln!(
                                    "local App Server connection shutdown handle failed: {error}"
                                );
                                continue;
                            }
                        };
                        let connection = match active_connections.register(shutdown_stream) {
                            Ok(connection) => connection,
                            Err(error) => {
                                eprintln!("local App Server connection tracking failed: {error}");
                                continue;
                            }
                        };
                        if let Err(error) = thread::Builder::new()
                            .name("zeta-local-app-server-connection".into())
                            .spawn(move || {
                                let _connection = connection;
                                if let Err(error) = server.serve_product_host_jsonl(reader, stream)
                                    && !is_peer_disconnect(&error)
                                {
                                    eprintln!("local App Server connection failed: {error}");
                                }
                            })
                        {
                            eprintln!("local App Server connection thread failed: {error}");
                        }
                    }
                }
            }
            Ok(LocalSocketAccept::Pending) => {
                if active_connections.is_empty() && registry.active_terminal_count() == 0
                    && !registry.automation_needs_host()? {
                    let idle_since = idle_since.get_or_insert_with(Instant::now);
                    if idle_since.elapsed() >= idle_timeout {
                        eprintln!("local App Server daemon exited after its idle timeout");
                        return Ok(());
                    }
                } else {
                    idle_since = None;
                }
                thread::sleep(IDLE_POLL_INTERVAL);
            }
            Ok(LocalSocketAccept::Rejected(error)) => {
                eprintln!("local App Server connection blocking mode failed: {error}");
            }
            Err(error) => return Err(format!("local App Server accept failed: {error}")),
        }
    }
}

fn exit_after_stop(
    listener: PollingLocalListener,
    socket_cleanup: SocketCleanup,
    process_record: ProcessRecordGuard,
) -> ! {
    eprintln!("local App Server daemon stopped");
    let _ = io::stderr().flush();
    // Directory runtimes can own worker threads whose destructors outlive the lifecycle deadline.
    // Remove the endpoint identity first, then terminate this dedicated daemon process without
    // waiting indefinitely for those runtime destructors.
    drop(listener);
    drop(socket_cleanup);
    drop(process_record);
    std::process::exit(0);
}

fn configured_idle_timeout() -> Result<Duration, String> {
    let Some(value) = std::env::var_os(IDLE_TIMEOUT_ENV) else {
        return Ok(DEFAULT_IDLE_TIMEOUT);
    };
    let value = value.to_string_lossy();
    let millis = value
        .parse::<u64>()
        .map_err(|_| format!("{IDLE_TIMEOUT_ENV} must be milliseconds"))?;
    if millis < IDLE_POLL_INTERVAL.as_millis() as u64 {
        return Err(format!(
            "{IDLE_TIMEOUT_ENV} must be at least {}",
            IDLE_POLL_INTERVAL.as_millis()
        ));
    }
    Ok(Duration::from_millis(millis))
}

#[cfg(unix)]
fn register_shutdown_signals(stop_requested: &Arc<AtomicBool>) -> Result<(), String> {
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register(signal, Arc::clone(stop_requested))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn register_shutdown_signals(_stop_requested: &Arc<AtomicBool>) -> Result<(), String> {
    Ok(())
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

fn is_peer_disconnect(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;
