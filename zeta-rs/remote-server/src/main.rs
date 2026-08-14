use std::process::ExitCode;

fn main() -> ExitCode {
    match zeta_remote_server::run_from_environment(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-remote-server: {error}");
            ExitCode::FAILURE
        }
    }
}
