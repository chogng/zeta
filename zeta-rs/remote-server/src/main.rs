use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(error) = process_hardening::initialize() {
        eprintln!("process hardening failed: {error}");
        std::process::exit(1);
    }

    match zeta_remote_server::run_from_environment(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-remote-server: {error}");
            ExitCode::FAILURE
        }
    }
}
