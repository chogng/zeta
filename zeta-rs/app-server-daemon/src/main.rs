use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(error) = process_hardening::initialize() {
        eprintln!("process hardening failed: {error}");
        std::process::exit(1);
    }

    let _package_lease = match std::env::current_exe()
        .and_then(zeta_package_store::acquire_package_lease_for_executable)
    {
        Ok(lease) => lease,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: could not lease its package: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = match arg0::dispatch(std::env::args_os().skip(1)) {
        Some(result) => result,
        None => {
            let arguments = std::env::args().skip(1).collect::<Vec<_>>();
            if arguments.is_empty()
                || arguments.as_slice() == [zeta_app_server_daemon::DAEMON_PROCESS_ARGUMENT]
            {
                zeta_app_server_daemon::run_from_environment(arguments)
            } else {
                std::env::current_exe()
                    .map_err(|error| error.to_string())
                    .and_then(|executable| {
                        zeta_app_server_daemon::run_command(arguments, &executable)
                    })
            }
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: {error}");
            ExitCode::FAILURE
        }
    }
}
