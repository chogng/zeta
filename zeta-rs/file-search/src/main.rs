mod cli;

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("zeta-file-search: {error}");
        std::process::exit(1);
    }
}
