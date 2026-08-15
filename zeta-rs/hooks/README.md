# `zeta-hooks`

> 本 README 负责声明式 Hook 运行时的实现契约。Core 安全点和执行顺序的跨系统语义见
> [`docs/core.md`](../../docs/core.md)，持久化声明与作用域解析见
> [`docs/config.md`](../../docs/config.md)。

`zeta-hooks` 在宿主已经授权的工作区中，把不可变 Hook 配置快照投影为经过动作策略评估的沙箱
进程。它拥有事件匹配、稳定执行顺序、动作身份、执行限制和运行时工作区绑定；不拥有配置持久化、
Core 安全点、工作区信任决策、外部协议或批准界面。

## 所有权与依赖方向

| 责任 | Owner | 本 crate 的边界 |
| --- | --- | --- |
| `beforeTool`、`afterTool`、`turnCompleted` 安全点 | `zeta-core` | 实现 `HookService`，不决定调用时机 |
| `HookId`、matcher、action 与 desired enablement | `zeta-config` | 消费完整 `HooksConfig` 快照，不读写配置文件 |
| Hook 匹配、动作评估与沙箱进程 | `zeta-hooks` | 唯一运行时 owner |
| 工作区信任与 `ExecuteProcess` capability | App Server / Workspace authority | 宿主授权后才能调用 `bind_workspace` |
| RPC DTO、revision-safe mutation 与错误投影 | App Server protocol / App Server | 本 crate 不依赖 transport |

依赖方向是 `zeta-hooks → zeta-core`，因为 `HookService` 是 Core 拥有的消费方端口；Core 不得反向
依赖本 crate。`zeta-hooks → zeta-config` 只消费无运行时状态的声明，不得把 PID、执行结果、队列或
重试状态写回 Config。

## 公共契约

`DeclarativeHookRuntime::new` 接收初始 `HooksConfig` 和宿主动作策略。调用方随后可以：

- 使用 `replace_config` 原子替换未来调用读取的声明快照；
- 在已经通过工作区信任检查后使用 `bind_workspace` 安装沙箱进程执行器；
- 使用 `unbind_workspace` 立即移除进程执行能力；
- 把 runtime 作为 `Arc<dyn zeta_core::HookService>` 注入 `TurnExecutor`。

`replace_config` 不改变正在执行的 invocation：`run_event` 在开始时克隆一份完整配置快照和当前
process binding。没有 workspace binding 时，事件成功地执行为空操作；这保证 Restricted Workspace
不会构造或保留进程执行器。

## 内部接口与调用关系

| Symbol | 职责 | 不得承担 |
| --- | --- | --- |
| `DeclarativeHookRuntime::run_event` | 观察取消、冻结调用快照、按 `BTreeMap` identity 顺序匹配并逐个执行 | 不读取 mutable Config authority 或重新安排 Core 安全点 |
| `matches_event` | 把 Core runtime event 与 Config declaration event/tool matcher 对齐 | 不解释 tool output 或添加隐式 glob/regex 语义 |
| `review_request` | 将 Hook ID、program、arguments 与 canonical workspace 绑定为 exact action digest、来源和能力集合 | 不授予执行权限 |
| `HookProcessExecutor` | 隔离可测试的工作区进程执行 seam | 不成为公共插件扩展面 |
| `NativeHookProcessExecutor` | 用统一 `CommandExecutor`、原生 sandbox 和固定限制执行 process action | 不自行读取信任配置或放宽策略决定 |
| `hook_execution_error` | 将 executor failure 收敛为不泄漏 child stderr 的 Core failure | 不把内部 sandbox 文本返回给产品客户端 |

```text
Core Hook safe point
└─ HookService::run
   └─ DeclarativeHookRuntime::run_event
      ├─ clone HooksConfig + workspace process binding
      ├─ matches_event
      ├─ review_request
      ├─ ActionPolicyService::decide
      └─ HookProcessExecutor::execute
         └─ CommandExecutor → native sandbox → process
```

## 安全与失败语义

- 每个 Hook 动作摘要绑定 Hook ID、完整 argv 和 canonical workspace；授权凭证必须再次匹配动作摘要、
  能力集合与策略版本。
- 默认沙箱允许工作区读写、拒绝网络，并把 process spawn capability 绑定到声明的 program。
- 单个进程最长运行 30 秒，captured output 上限为 64 KiB；非零退出码是执行失败。
- `AskUser` 不会从后台 Hook 打开交互式批准，而是失败关闭；block、revision mismatch 与错误 grant
  同样不能执行。
- 取消在事件开始、每个 Hook 之前和进程执行期间观察。Core 在 durable Turn completion 后忽略
  `turnCompleted` failure；`beforeTool` 与 `afterTool` failure 会返回给 Tool scheduler。
- `HookWorkspaceBindingError` 只表示无法为已授权工作区构造原生 sandbox；信任拒绝应由宿主在调用
  `bind_workspace` 之前处理。

## 验证与修改影响

```text
cargo test -p zeta-hooks
bazel test //zeta-rs/hooks:hooks-unit-tests
```

测试覆盖 stable identity ordering、event/tool matcher、disabled declaration、调用间取消和 exact Hook
action identity。修改事件种类时同步检查 `zeta-core` 安全点、`zeta-config` declaration、App Server
DTO/schema 与本文档；修改 action shape、capability 或 sandbox policy 时同步检查
`zeta-action-policy`、`zeta-tool-executor` 和权限文档。

当前只支持 process action，以及 macOS、Linux 和 Windows 原生 sandbox。并行 Hook、retry、持久化
execution record、环境变量声明和网络 capability 都尚未实现；增加这些能力必须先定义 durable、
policy 与 secret boundary，不能直接扩展 `NativeHookProcessExecutor` 绕过领域设计。
