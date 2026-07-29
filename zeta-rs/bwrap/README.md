# `zeta-bwrap`

> 本 README 拥有 Bubblewrap argv construction 与 Cargo-built upstream wrapper 的实现契约；
> shared sandbox policy 和平台选择见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)，上游 source provenance 见
> [`third_party/bubblewrap`](../../third_party/bubblewrap/README.md)。

本 crate 有两个彼此分离的 surface：

| Surface | 所有权 | 不承担 |
| --- | --- | --- |
| library `BwrapCommandBuilder` | typed mount、namespace、cwd 与 inner argv construction | policy、进程启动、binary discovery |
| binary `bwrap` | 把 Cargo argv 传给锁定 upstream C `bwrap_main` | Zeta policy、fallback、capability probe |

`src/builder.rs` 只生成 `BwrapCommand`，从不经过 shell。`build.rs` 仅在 Linux target 且设置
`ZETA_BWRAP_SOURCE_DIR` 时编译 `bubblewrap.c`、`bind-mount.c`、`network.c` 与 `utils.c`，并把
upstream `main` 重命名为 `bwrap_main`。普通 workspace test 不设置该变量，因此不会隐式下载或
编译 C；canonical package builder 先验证 source lock，再提供该目录。

`bwrap` binary 也是 repository 中刻意收窄的 unsafe-code 例外：`src/main.rs` 仅负责把
process arguments 转换后调用 upstream C entry point。用于构造 Bubblewrap argv 的 library
仍遵守 workspace 的 `unsafe_code = "forbid"`。

```text
scripts/zeta_package/bubblewrap.py
→ verify/extract locked upstream source
→ ZETA_BWRAP_SOURCE_DIR
→ build.rs
→ static C library exposing bwrap_main
→ src/main.rs
→ zeta-resources/bwrap
```

新增 upstream source file、feature define 或 linker dependency 时，必须同步 source lock、
`build.rs`、package tests、Bubblewrap notices 和 Linux release toolchain。把 policy resolution、
executable selection 或 process execution加入本 crate 表示 ownership 漂移。

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-bwrap
cargo clippy --manifest-path zeta-rs/Cargo.toml \
  -p zeta-bwrap --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/bwrap:bwrap-unit-tests
```
