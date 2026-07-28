# `zeta-shell-command`

> 本 README 是显式本地进程 Tool 与只读 ripgrep profile 的实现契约。跨 crate Tool ownership
> 见 [`docs/tools.md`](../../docs/tools.md)，sandbox policy 见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。

本 crate 拥有 `shell-command` host definition、结构化请求解析和冻结的 `rg` executable
materialization。它不隐式启动 shell，不决定 Core approval，也不持有 Thread/Turn 状态。

## 文件与职责

| 文件 | 职责 |
| --- | --- |
| `src/lib.rs` | `ShellCommandTool`、`ShellCommandRequest`、definition 与 executor bridge |
| `src/ripgrep.rs` | `RipgrepExecutable` discovery、identity freeze 和只读参数约束 |
| `src/shell_command_tests.rs` | binding、authority、sandbox denial 与 validation |
| `src/ripgrep_tests.rs` | executable discovery 和 unsafe flag rejection |

## Public contract 与调用路径

```text
ShellCommandTool::new
└─ shell_command_definition + zeta_exec::CommandExecutor

ToolExecutor::execute
├─ validate binding / environment / cancellation
├─ ShellCommandRequest::from_arguments
└─ ShellCommandTool::execute_authorized
   └─ CommandExecutor::execute

App Server local adapter
├─ RipgrepExecutable::discover
├─ ShellCommandRequest::from_arguments
├─ workspace argument validation
├─ RipgrepExecutable::materialize
└─ ShellCommandTool::execute_authorized
```

`ShellCommandRequest` 始终是显式 `program + arguments + relative working_directory`；没有 quoting、
globbing、environment expansion 或 shell parsing。`ShellCommandTool::execute_authorized` 只供已经
完成 host materialization 和 Core policy decision 的 adapter 使用。

## 只读 `rg` profile

`RipgrepExecutable::discover` 按以下顺序选择一次并 canonicalize：

1. `ZETA_RG_PATH`；
2. 当前 Zeta executable 同目录的 `rg`/`rg.exe`；
3. 启动时 `PATH`。

显式 override 无效时直接失败，不回退到其他 candidate。`materialize` 只接受模型别名 `rg`，
替换成冻结的绝对 executable，并在 argv 前加入 `--no-config`。preprocessor、archive search、
symlink follow、pattern file 与 ignore file 参数会被拒绝，因为它们会扩大进程或文件读取边界。
App Server adapter 还拒绝绝对路径和含 `..` component 的参数。
已存在的相对 path argument 会在 review 前 canonicalize；通过 Workspace 内 symlink 指向外部的
路径同样被拒绝。

当前 discovery 不等于 bundled distribution：仓库尚未打包 `rg`，也没有执行版本/capability
probe。Packaging 可以把兼容 binary 放到 Zeta executable 同目录，但不能改变上述 identity freeze
和 fail-closed 语义。

## Failure、取消与输出

- JSON shape、空 program、binding 或 environment 不匹配：子进程启动前失败；
- Core 选择的 sandbox backend 无法建立边界：返回 structured `SandboxDenied`；
- cancellation 在 spawn 前返回 not-started；spawn 后终止 child 并返回 uncertain outcome；
- timeout 同样终止 child，并按 started/uncertain 处理；
- stdout/stderr 共用一个 byte budget，结果保留 bounded prefix，并分别标记是否截断。

底层 checkpoint、kill 和 capture 语义由 [`zeta-exec`](../exec/README.md) canonical 定义。

## 修改与验证

修改 model schema、危险 `rg` flags、discovery 顺序或 outcome mapping 时，必须同步检查 App Server
`local_tools.rs`、本 README 和 `docs/tools.md`。

```bash
cargo test -p zeta-shell-command
cargo clippy -p zeta-shell-command --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/shell-command:shell-command-unit-tests
```
