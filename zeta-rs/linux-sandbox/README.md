# `zeta-linux-sandbox`

> 本 README 拥有共享沙箱策略到 Linux Bubblewrap 强制执行的实现契约；随包源码见 [`zeta-rs/vendor/bubblewrap`](../vendor/bubblewrap/README.md)，跨平台决策见 [`docs/sandboxing.md`](../../docs/sandboxing.md)。

`LinuxSandbox::discover` 在主机组合时冻结 Bubblewrap 可执行文件。候选由 `zeta-install-context` 提供：

1. authoritative `ZETA_BWRAP_PATH`；
2. package `zeta-resources/bwrap`；
3. 启动时 host `PATH` 中的 `bwrap`。

私有函数 `discovery::validate_candidate` 规范化可执行文件路径，并拒绝非普通文件和不可执行文件；`probe_bubblewrap` 执行 `--help`，要求实际二进制支持参数构造器使用的挂载、只读挂载、网络命名空间、父进程退出和新会话参数。显式覆盖无效时直接报错；普通候选可以跳过并继续。选中的规范路径进入 `LinuxSandbox::bwrap_binary`，探测与执行不会重新解析 `PATH`。

`LinuxSandbox::prepare_command` 根据 `SandboxPolicy` 生成 `PreparedCommand`：非完全文件访问从只读根目录开始，工作区可写模式叠加可写工作区和只读保护元数据，禁止网络时添加独立网络命名空间。私有 `bwrap::BwrapCommandBuilder` 负责生成结构化参数；`classify_denial` 区分 Bubblewrap 设置失败与进程可能已启动后的操作系统拒绝。

```text
InstallContext::executable_candidates(Bubblewrap)
→ discovery::validate_candidate
→ discovery::probe_bubblewrap
→ LinuxSandbox { canonical bwrap_binary }
→ prepare_command
→ bwrap::BwrapCommandBuilder
→ CommandExecutor
```

当前限制：尚未加入 seccomp、WSL 专用诊断或受管网络桥接。修改能力标记、挂载顺序、保护元数据、参数构造或拒绝分类时，必须同步本 crate 测试、发布包契约和系统文档。

```bash
cargo test --manifest-path Cargo.toml -p zeta-linux-sandbox
cargo clippy --manifest-path Cargo.toml \
  -p zeta-linux-sandbox --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/linux-sandbox:linux-sandbox-unit-tests
```
