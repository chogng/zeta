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
    let result = zeta_app_server_daemon::backend_executable_path().and_then(|executable| {
        zeta_app_server_daemon::run_command(std::env::args().skip(1), &executable)
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: {error}");
            ExitCode::FAILURE
        }
    }
}
