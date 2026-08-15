use std::process::ExitCode;

fn main() -> ExitCode {
    match zeta_server_host::run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zeta-server: {error}");
            ExitCode::FAILURE
        }
    }
}
