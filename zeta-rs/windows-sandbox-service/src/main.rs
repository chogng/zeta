use anyhow::Result;

fn main() -> Result<()> {
    zeta_windows_sandbox_service::run(std::env::args().skip(1))
}
