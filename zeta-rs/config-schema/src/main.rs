fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [] => print!("{}", zeta_config_schema::generate()),
        [path] => std::fs::write(path, zeta_config_schema::generate())?,
        _ => return Err("usage: zeta-config-schema [output.json]".into()),
    }
    Ok(())
}
