use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use zeta_collaboration_server::CollaborationServerOptions;
use zeta_collaboration_server::run;

fn main() {
    if let Err(error) = run_from_environment() {
        eprintln!("zeta-collaboration-server: {error}");
        std::process::exit(1);
    }
}

fn run_from_environment() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let [listen, database] = arguments.as_slice() else {
        return Err("usage: zeta-collaboration-server <IP:PORT> <DATABASE_PATH>; requires ZETA_COLLABORATION_BEARER_TOKEN and optionally ZETA_COLLABORATION_ALLOWED_ORIGIN".into());
    };
    let address: SocketAddr = listen
        .parse()
        .map_err(|_| "Collaboration listener must be an IP:PORT pair")?;
    let token = env::var("ZETA_COLLABORATION_BEARER_TOKEN")
        .map_err(|_| "ZETA_COLLABORATION_BEARER_TOKEN is required")?;
    let mut options = CollaborationServerOptions::new(address, PathBuf::from(database), token);
    if let Ok(origins) = env::var("ZETA_COLLABORATION_ALLOWED_ORIGIN") {
        for origin in origins.split(',').filter(|origin| !origin.is_empty()) {
            options = options.with_allowed_origin(origin);
        }
    }
    run(options)?;
    Ok(())
}
