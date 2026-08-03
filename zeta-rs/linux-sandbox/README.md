# `zeta-linux-sandbox`

> 本 README 拥有 shared sandbox policy 到 Linux Bubblewrap enforcement 的实现契约；
> package source/build contract 见
> [`scripts/zeta_package`](../../scripts/zeta_package/README.md)，跨平台决策见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。

`LinuxSandbox::discover` 在 host composition 时冻结 Bubblewrap executable。候选由
`zeta-install-context` 提供：

1. authoritative `ZETA_BWRAP_PATH`；
2. package `zeta-resources/bwrap`；
3. 启动时 host `PATH` 中的 `bwrap`。

私有 `discovery::validate_candidate` canonicalize executable、拒绝非普通文件和非 executable；
`probe_bubblewrap` 执行 `--help`，要求实际 binary 支持 builder 使用的 bind、read-only bind、
network namespace、parent-death 与 new-session flags。显式 override 无效时不 fallback；普通候选
可以跳过并继续。选中的 canonical path 进入 `LinuxSandbox::bwrap_binary`，probe 与执行不会重新
解析 `PATH`。

`LinuxSandbox::prepare_command` 根据 `SandboxPolicy` 生成 `PreparedCommand`：非 FullAccess
root read-only，WorkspaceWrite 叠加 writable Workspace 和 read-only protected metadata，
Denied network 添加独立 network namespace。`classify_denial` 区分 Bubblewrap setup failure 与
可能已启动进程后的 OS denial。

```text
InstallContext::executable_candidates(Bubblewrap)
→ discovery::validate_candidate
→ discovery::probe_bubblewrap
→ LinuxSandbox { canonical bwrap_binary }
→ prepare_command
→ zeta_bwrap::BwrapCommandBuilder
→ CommandExecutor
```

当前限制：尚未加入 seccomp、WSL-specific diagnostics 或 managed-network bridge。修改 capability
markers、mount order、protected metadata 或 denial classification 时，必须同步本 crate tests、
`zeta-bwrap` builder tests、package contract 和系统文档。

```bash
cargo test --manifest-path Cargo.toml -p zeta-linux-sandbox
cargo clippy --manifest-path Cargo.toml \
  -p zeta-linux-sandbox --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/linux-sandbox:linux-sandbox-unit-tests
```
