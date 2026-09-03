use std::process::ExitCode;

fn main() -> ExitCode {
    let _package_lease = match std::env::current_exe()
        .and_then(zeta_package_store::acquire_package_lease_for_executable)
    {
        Ok(lease) => lease,
        Err(error) => {
            eprintln!("zeta-server: could not lease its package: {error}");
            return ExitCode::FAILURE;
        }
    };
    match zeta_server_host::run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-server: {error}");
            ExitCode::FAILURE
        }
    }
}
