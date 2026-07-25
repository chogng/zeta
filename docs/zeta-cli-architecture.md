# Zeta CLI 架构与协作边界

> 负责人：CLI 开发者  
> App Server 与 Rust 对接负责人：zeta-rs 开发者  
> CLI 与 Desktop 共用唯一的 Zeta App Server 产品契约。
> 当前开发基线：[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md)

## 1. 目标

CLI 是正式产品客户端。它负责终端体验，不拥有 Agent 状态机，也不建立第二套 Rust 业务
facade。

第一版命令：

```bash
zeta
zeta ask "解释当前仓库"
zeta exec "检查测试失败"
zeta login
zeta config
zeta app-server --listen stdio://
```

`ask` 和 `exec` 都通过 App Server 的 Thread、Turn 和事件接口工作。`exec` 是非交互 Agent
入口，不是绕过 Agent Tool Executor 的任意 shell 执行器。

## 2. 物理位置与所有权

CLI 保留在同一个 Rust workspace：

```text
zeta-rs/
├── app-server/
├── app-server-client/
├── app-server-protocol/
├── tui/
└── cli/
```

CLI 开发者负责：

- 参数解析、子命令和帮助；
- Human、JSON 和 JSONL 输出；
- TTY 检测、颜色、进度、退出码；
- TUI 交互和键盘事件；
- shell completion、安装体验和 CLI 集成测试。

CLI 开发者不负责：

- Thread、Turn、Item 和 Tool Call 状态机；
- rollout、SQLite、writer lease；
- sandbox、审批和工具执行策略；
- 模型供应商内部实现；
- App Server wire DTO 的权威定义；
- Desktop 或 Browser View。

## 3. 唯一产品接口

普通 CLI 路径依赖：

```text
zeta-cli
  → zeta-app-server-client
  → zeta-app-server-protocol
```

CLI 不直接调用 `zeta-core`、`zeta-storage`、`zeta-exec`、`zeta-sandboxing` 或 Model Provider。

`zeta app-server` 子命令可以作为明确的宿主入口依赖 `zeta-app-server`，但不能绕过
dispatcher 直接调用 Core 用例。

禁止新增名为 `runtime`、`service`、`common` 或 `platform` 的泛化 CLI facade 来聚合内部
能力。这类层会重复 App Server 契约并逐渐成为职责不清的杂物桶。

## 4. Transport 模式

所有模式使用相同的 Params、Result、Notification 和 Error DTO：

```text
默认 CLI/TUI
  → InProcessAppServerClient
  → 同进程 App Server dispatcher

Desktop
  → JSONL / stdio
  → 独立 App Server

本地 daemon
  → Unix socket
  → App Server daemon

远程 CLI/TUI
  → WebSocket
  → 远程 App Server
```

进程内模式可以避免子进程和序列化开销，但仍必须通过同一个 typed client 和 dispatcher，
不能提供只在进程内可用的隐藏方法。

## 5. 请求与事件

CLI 需要的能力必须先进入 App Server API 文档，再由 zeta-rs 实现：

- `initialize`
- Thread start/read/resume/list/unsubscribe
- Turn start/steer/interrupt
- Agent message delta/completed
- Tool Call proposed/running/completed
- Approval request/response
- 文件变更
- warning
- Turn completed/failed/interrupted

CLI 不解析日志、stderr 或人类文本来判断状态。

## 6. 输出契约

Human 输出可以演进；JSON/JSONL 是稳定 CLI 契约，但其事件必须由 App Server typed
notification 显式映射。

```json
{
  "type": "item.agentMessage.delta",
  "threadId": "thread_123",
  "turnId": "turn_456",
  "itemId": "item_789",
  "streamSeq": 14,
  "delta": "..."
}
```

stderr 只用于诊断。stdout 在 JSON/JSONL 模式下只能输出机器数据。

建议退出码：

```text
0  成功
1  一般运行失败
2  参数、配置或协议错误
3  审批被拒绝
4  用户中断
5  capability 不可用
6  Thread writer lease 冲突
```

## 7. 审批与安全

CLI 可以展示审批 UI，但是否需要审批由 Core policy 决定，并经 App Server 双向请求送达
CLI。

审批响应必须绑定：

- `approvalRequestId`
- `threadId`
- `turnId`
- `toolCallId`
- `actionDigest`
- decision、scope 和 expiry

CLI 不能直接执行 Agent 请求的命令、写文件或网络操作；这些动作必须经过 Rust Tool
Executor 和 host policy。

## 8. 协作交接

zeta-rs 开发者交付：

- 版本化 `zeta-app-server-protocol`；
- `zeta-app-server-client` 的进程内与远程实现；
- typed request、notification 和 error；
- mock transport、fixtures 和 contract tests；
- error → exit code 建议映射。

CLI 开发者交付：

- 命令、参数和输出格式文档；
- 所需 App Server method/notification 清单；
- 每个命令的输入/输出 fixture；
- TTY 与非 TTY 行为；
- approval 交互流程；
- CLI 集成测试。

CLI 新需求不得通过直接依赖内部 crate 临时解决；先补产品 API 契约。

当前可以实现的 method、notification 和限制以
[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md) 为准。

## 9. 验收

- `ask`、`exec`、`login`、`config` 通过 App Server Client 工作；
- 默认进程内模式仍经过 JSON-RPC dispatcher；
- CLI 和 Desktop 对相同请求得到相同 DTO 与错误语义；
- CLI crate 不直接依赖 Core、Storage、Exec、Sandbox 或 Model Provider；
- JSON/JSONL stdout 无日志污染；
- Ctrl-C 发出 `turn/interrupt` 并等待终态；
- TTY、pipe、重定向和非交互环境均有测试。
