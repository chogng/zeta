use super::*;

#[test]
fn mcp_http_listener_requires_an_ip_port_and_endpoint_path() {
    let (address, path) = parse_mcp_http_address("http://127.0.0.1:8787/mcp").unwrap();
    assert_eq!(address, "127.0.0.1:8787".parse().unwrap());
    assert_eq!(path, "/mcp");
    assert!(parse_mcp_http_address("https://127.0.0.1:8787/mcp").is_err());
    assert!(parse_mcp_http_address("http://localhost:8787/mcp").is_err());
    assert!(parse_mcp_http_address("http://127.0.0.1:8787").is_err());
}
