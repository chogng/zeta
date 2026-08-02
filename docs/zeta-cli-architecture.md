# Zeta CLI 架构与协作边界

> 负责人：CLI 开发者  
> App Server 与 Rust 对接负责人：zeta-rs 开发者  
> CLI 与 Desktop 共用唯一的 Zeta App Server 产品契约。
> 当前开发基线：[`zeta-app-server-api.md`](zeta-app-server-api.md)
> 本地启动与连接基线：[`app-server-client.md`](app-server-client.md)
> 无交互执行基线：[`exec.md`](exec.md)
> MCP Agent server：[`mcp-server.md`](mcp-server.md)
> 三条公开产品线与宿主边界：[`product-lines.md`](product-lines.md)

## 快速理解

`zeta code` 是 Zeta 的 TUI 产品线；`zeta-cli` 是它的命令入口和宿主启动器，`zeta-tui` 负责
交互呈现。CLI 负责命令、输入输出和退出码，但所有 Agent 工作都通过统一的 Session-first
App Server 契约执行。

| 用户命令 | CLI 负责 | 后端负责 |
| --- | --- | --- |
| `zeta` | 启动交互式终端体验 | Session、Thread 和 Agent 状态 |
| `zeta ask` | 解析参数、创建工作并流式显示更新 | 模型、工具、权限和持久化 |
| `zeta exec` | 提供无交互输入和机器可消费输出 | 执行同一 Agent/Tool 路径，不绕过工具系统 |
| `zeta login` / `zeta config` | 收集用户意图并调用类型化方法 | 登录、秘密和配置权威 |
| `zeta app-server` / `zeta mcp-server` | 选择进程入口和监听方式 | 对应服务的协议与生命周期 |
| 连接中断或失败 | 输出稳定错误、诊断和退出码 | 决定领域失败和恢复语义 |

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
zeta mcp-server
zeta mcp-server --listen http://127.0.0.1:8787/mcp
```

`ask` 和 `exec` 当前通过 Session-first API 工作：先创建 Session，再在其中创建 Thread，
最后启动 Turn 并消费 canonical update。`exec` 是
非交互 Agent 入口，不是绕过 Agent Tool Executor 的任意 shell 执行器。

## 2. 物理位置与所有权

CLI 保留在同一个 Rust workspace：

```text
zeta-rs/
├── app-server/
├── app-server-client/
├── app-server-protocol/
├── exec/                # target: headless Agent runner
├── tool-executor/       # target: current process executor migration
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

- Session、Thread、Turn、ThreadItem 和 Tool Call 状态机；
- rollout、SQLite、writer lease；
- sandbox、审批和工具执行策略；
- 模型供应商内部实现；
- App Server wire DTO 的权威定义；
- Desktop 或 Browser View。

## 3. 唯一产品接口

普通 CLI 路径分为：

```text
zeta-cli → zeta-tui  → zeta-app-server-client
         └→ zeta-exec → zeta-app-server-client
                                ↓
                       zeta-app-server-protocol
```

CLI 可以依赖产品层 `zeta-exec` 运行无交互 Agent，但不依赖目标
`zeta-tool-executor`。CLI/TUI/exec 都不直接调用 `zeta-core`、`zeta-storage`、
`zeta-rollout`、`zeta-rollout-trace`、`zeta-sandboxing` 或 Model Provider。

`zeta app-server` 子命令可以作为明确的宿主入口依赖 `zeta-app-server`，但不能绕过
dispatcher 直接调用 Core 用例。

`zeta mcp-server` 是 `zeta-mcp-server` binary 的 CLI 入口。该 crate 依赖
`zeta-app-server-client`，将外部 MCP tool call 映射到同一 Session/Thread/Turn dispatcher；
CLI 不复制 MCP framing、Agent polling、interaction 或 invocation identity 逻辑。CLI 只解析
`stdio://` 或 `http://IP:PORT/PATH` listener；HTTP bearer 由
`ZETA_MCP_BEARER_TOKEN` 提供，可选 exact Origin 由 `ZETA_MCP_ALLOWED_ORIGIN` 提供。非 loopback
远程部署必须在 TLS/auth reverse proxy 后运行，CLI 本身不拥有 OAuth 或 tenant provisioning。

禁止新增名为 `runtime`、`service`、`common` 或 `platform` 的泛化 CLI facade 来聚合内部
能力。这类层会重复 App Server 契约并逐渐成为职责不清的杂物桶。

## 4. 传输模式

所有模式使用相同的 Params、Result、Notification 和 Error DTO：

```text
默认 CLI/TUI
  → AppServerSession
  → request handle + event stream
  → typed in-process App Server dispatcher

Desktop
  → JSONL / stdio
  → 独立 App Server

未来 daemon/remote App Server
  → App Server Client remote backend
  → 相同 App Server protocol

未来 remote scheduler
  → zeta-exec worker adapter
  → App Server Client
```

进程内模式可以避免子进程和 JSON 编解码，但仍必须通过同一个 typed client 和 dispatcher，
不能提供只在进程内可用的隐藏业务方法。当前 `zeta-app-server-client` 的首要职责是为
`zeta-exec` 和 TUI 启动本地 App Server、完成 initialize、连接 request/event channel 并正确
关闭。后续 `zeta-exec` 作为远程调度的 headless execution entry；scheduler adapter 仍通过
这一 owned session 工作，不能建立第二套 Core 或 App Server 私有调用路径。完整 Job/Attempt、
lease、event cursor 与 remote execution plane 边界见 [`exec.md`](exec.md)。

## 5. 请求与事件

CLI 需要的能力必须先进入 App Server API 文档，再由 zeta-rs 实现：

- `initialize`
- Session create/read/list/subscribe/lifecycle
- Session-owned Thread create/fork/archive
- Thread read/subscribe/unsubscribe
- Turn start/interrupt
- SessionUpdate / ThreadUpdate
- Tool Call proposed/running/completed
- Approval request/response
- 文件变更
- warning
- Turn completed/failed/interrupted

CLI 不解析日志、stderr 或人类文本来判断状态。

## 6. 输出契约

Human 输出可以演进；JSON/JSONL 是稳定 CLI 契约，但其事件必须由 App Server typed
notification 显式映射。具体 mapping、stdout/stderr 和 scheduler event 规则由
[`exec.md`](exec.md#7-输出契约) 维护。

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

CLI/TUI 可以展示审批 UI；headless exec 必须使用明确的 deny/configured/delegated policy。
是否需要审批由 Core policy 决定，并经 App Server 双向请求送达 consumer。

审批响应必须绑定：

- `approvalRequestId`
- `threadId`
- `turnId`
- `toolCallId`
- `actionDigest`
- decision、scope 和 expiry

CLI 不能直接执行 Agent 请求的命令、写文件或网络操作；这些动作必须经过 Rust Tool
Executor 和 host policy。目标底层边界名为 `zeta-tool-executor`，不能与产品层
`zeta-exec` 混用。

## 8. 协作交接

zeta-rs 开发者交付：

- 版本化 `zeta-app-server-protocol`；
- `zeta-app-server-client` 的本地 App Server owned session、request handle 与 event stream；
- typed request、notification 和 error；
- mock transport、fixtures 和 contract tests；
- error → exit code 建议映射。

`zeta-exec` 开发者交付：

- run-once、resume、interrupt 与 terminal outcome；
- human/JSONL output contract；
- headless approval handling；
- 后续 remote worker 的 Job/Attempt/lease/cursor adapter。

CLI 开发者交付：

- 命令、参数和输出格式文档；
- 所需 App Server method/notification 清单；
- 每个命令的输入/输出 fixture；
- TTY 与非 TTY 行为；
- approval 交互流程；
- CLI 集成测试。

CLI 新需求不得通过直接依赖内部 crate 临时解决；先补产品 API 契约。

当前可以实现的 method、notification 和限制以
[`zeta-app-server-api.md`](zeta-app-server-api.md) 为准。

## 9. 验收

- `ask`、`exec`、`login`、`config` 通过 App Server Client 工作；
- 默认进程内模式通过 typed channel 经过相同 App Server method dispatcher；
- `zeta-exec` 与 TUI 共用 App Server 启动、initialize、channel wiring 和 shutdown 实现；
- 远程调度通过 `zeta-exec` 映射为相同 typed request 与 canonical update；
- CLI 和 Desktop 对相同请求得到相同 DTO 与错误语义；
- CLI crate 不直接依赖 Core、Storage、Tool Executor、Sandbox 或 Model Provider；
- JSON/JSONL stdout 无日志污染；
- Ctrl-C 发出 `turn/interrupt` 并等待终态；
- TTY、pipe、重定向和非交互环境均有测试。
