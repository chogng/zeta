# Patched `candle-onnx`

This directory contains the source required to build `candle-onnx` 0.9.2 from
[Hugging Face Candle](https://github.com/huggingface/candle), upstream commit
`3b39794c14b1de8ba4c2d10d49354d557a415547`.

Zeta changes only the build-time Protobuf setup: `build.rs` resolves `protoc`
through `protoc-bin-vendored` and passes that executable directly to
`prost-build`. This keeps `cargo build` independent of a host-installed
`protoc`; the compiler is not included in the Zeta application at runtime.

The upstream source is available under Apache-2.0 or MIT. This copy is
distributed under Apache-2.0; see `LICENSE-APACHE`.
