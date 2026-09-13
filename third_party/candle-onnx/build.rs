// Modified by Ash to use Cargo-provided protoc instead of a host installation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc);
    config.compile_protos(&["src/onnx.proto3"], &["src/"])?;
    Ok(())
}
