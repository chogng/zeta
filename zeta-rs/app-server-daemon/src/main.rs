use std::process::ExitCode;

fn main() -> ExitCode {
    let _package_lease = match std::env::current_exe()
        .and_then(zeta_package_store::acquire_package_lease_for_executable)
    {
        Ok(lease) => lease,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: could not lease its package: {error}");
            return ExitCode::FAILURE;
        }
    };
    match zeta_app_server_daemon::run_from_environment(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: {error}");
            ExitCode::FAILURE
        }
    }
}
