use std::env;
use std::path::PathBuf;
use zeta_mcp_server::{HttpServerOptions, McpServerOptions, run_http, run_stdio};

fn main() {
    if let Err(error) = run() {
        eprintln!("zeta-mcp-server: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let state_root = env::var_os("ZETA_STATE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".zeta"));
    let workspace_root = env::var_os("ZETA_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(env::current_dir)?;
    let options = McpServerOptions::new(state_root, workspace_root);
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => run_stdio(options)?,
        [listen, address] if listen == "--listen" && address == "stdio://" => {
            run_stdio(options)?;
        }
        [listen, address] if listen == "--listen" => {
            let (socket, path) = parse_http_address(address)?;
            let token = env::var("ZETA_MCP_BEARER_TOKEN")
                .map_err(|_| "ZETA_MCP_BEARER_TOKEN is required for Streamable HTTP")?;
            let mut http = HttpServerOptions::new(socket, path, token);
            if let Ok(origin) = env::var("ZETA_MCP_ALLOWED_ORIGIN") {
                http = http.with_allowed_origin(origin);
            }
            run_http(options, http)?;
        }
        _ => {
            return Err("usage: zeta-mcp-server [--listen stdio://|http://IP:PORT/PATH]".into());
        }
    }
    Ok(())
}

fn parse_http_address(
    address: &str,
) -> Result<(std::net::SocketAddr, String), Box<dyn std::error::Error>> {
    let remainder = address
        .strip_prefix("http://")
        .ok_or("MCP HTTP listener must use http:// behind a TLS reverse proxy")?;
    let (authority, path) = remainder
        .split_once('/')
        .ok_or("MCP HTTP listener must include an endpoint path")?;
    let socket = authority
        .parse()
        .map_err(|_| "MCP HTTP listener authority must be an IP:PORT pair")?;
    Ok((socket, format!("/{path}")))
}
