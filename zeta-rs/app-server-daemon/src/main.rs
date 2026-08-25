use std::process::ExitCode;

fn main() -> ExitCode {
    match zeta_app_server_daemon::run_from_environment(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-app-server-daemon: {error}");
            ExitCode::FAILURE
        }
    }
}
