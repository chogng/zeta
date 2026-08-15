# `zeta-tool-executor`

> 本 README 是 Zeta 单进程执行边界的实现契约。Tool ownership 见
> [`docs/tools.md`](../../docs/tools.md)，平台 enforcement 见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。完整 Agent 的无界面运行由
> [`zeta-exec`](../exec/README.md) 拥有。

本 crate 在 approval、WorkspaceRoot、sandbox backend、timeout、cancellation 和 output budget
都已固定后启动一个显式进程。它不解析 shell，不决定 Core policy，也不把普通 non-zero exit
自动分类为 sandbox denial。

## 公共契约

| Symbol | 职责 |
| --- | --- |
| `ApprovalPolicy` | 对 exact program/argv digest 给出 start gate |
| `CommandExecutionAuthority` | 显式区分 sandboxed 与 unrestricted execution |
| `ExecutionLimits` | 固定 wall-clock timeout 与 stdout/stderr 总 byte budget |
| `CommandInput` | 显式选择关闭 stdin 或写入调用方已经限制的 bytes |
| `CommandExecutor::execute` | prepare、spawn、监督、capture 与 backend denial classification |
| `CommandExecutionOutcome` | completed output 或 structured sandbox denial |
| `ExecutionError` | start 前拒绝、spawn failure、取消、timeout 或 sandbox setup failure |

真实顺序不可交换：

```text
cancellation checkpoint
→ ApprovalPolicy
→ cancellation checkpoint
→ SandboxManager::prepare
→ cancellation checkpoint
→ spawn prepared command
→ dedicated stdin writer + stdout/stderr readers
→ poll child + cancellation + timeout
→ join stdin/stdout/stderr workers
→ bounded merge
→ SandboxBackend::classify_denial
```

只有 backend 返回的 `PreparedCommand` 会被启动。requested command 不会绕过 backend 直接 spawn。
`Sandboxed` authority 若无法建立 backend boundary，必须失败关闭；调用方不能静默改成
`Unrestricted`。

## stdin、取消、timeout 与输出

`CommandInput::Closed` 把子进程 stdin 接到 null device；`CommandInput::Bytes` 在独立线程写入调用方
提供的完整 bytes，并在写入完成后关闭 pipe。调用方拥有 input budget，本 crate 不截断 stdin。独立
writer 保证不读取 stdin 的 child 不会阻止 cancellation/timeout 监督；终止路径会 kill/wait child
并 join writer 和两个 reader。

spawn 前观察到 cancellation 返回 `CancelledBeforeStart`。spawn 后 cancellation 或 timeout 会
先 kill/wait child，再 join I/O threads；前者返回 `CancelledAfterStart`，后者返回
`TimedOut`。上层必须把两者视为已经跨过 side-effect boundary，不能自动 replay。

stdout 与 stderr reader 各自最多读取总 budget，最终合并时 stdout 优先、stderr 使用剩余空间。
`CommandOutput::{stdout_truncated,stderr_truncated}` 明确指出原 stream 是否超过保留范围。budget 是
byte 数；截断跨 UTF-8 code point 时使用 lossy decoding，调用方不能把 retained string 当作完整
输出。

## 内部接口与漂移检查

`check_cancellation_before_start` 固定 pre-spawn semantics；`terminate` 统一 kill/wait/join 全部 I/O
worker；`drain_stream` 只负责 bounded capture。若 Tool schema、Core approval、自动 retry 或 Thread
mutation 进入本 crate，说明 ownership 已漂移。

```bash
cargo test -p zeta-tool-executor
cargo clippy -p zeta-tool-executor --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/tool-executor:tool-executor-unit-tests
```
