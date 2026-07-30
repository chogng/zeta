# `zeta-mcp-server`

> 本 README 负责当前 crate 的实现契约。跨 crate 产品架构和分阶段演进见
> [`docs/mcp-server.md`](../../docs/mcp-server.md)；反方向的 MCP 客户端运行时见
> [`docs/mcp.md`](../../docs/mcp.md)。

`zeta-mcp-server` 通过 stdio 或已认证的 Streamable HTTP，把 Zeta Agent 执行能力暴露给 MCP
客户端。它是 `zeta-app-server-client` 上方的适配器；嵌入式 App Server 仍是唯一组合根，也是
Session、Thread、Turn、模型、工具、策略、工作区和持久化状态的权威所有者。

## 1. 当前接口面

启动本地 stdio 服务：

```text
zeta-mcp-server
zeta mcp-server
zeta mcp-server --listen stdio://
```

启动 HTTP 端点：

```text
ZETA_MCP_BEARER_TOKEN=<at-least-32-visible-ASCII-characters> \
  zeta mcp-server --listen http://127.0.0.1:8787/mcp
```

两个入口都读取 `ZETA_STATE_ROOT` 和 `ZETA_WORKSPACE_ROOT`，默认值分别为 `.zeta` 和进程工作
目录。HTTP 还必须提供 `ZETA_MCP_BEARER_TOKEN`；`ZETA_MCP_ALLOWED_ORIGIN` 可以额外允许一个
精确浏览器来源。内置监听器只提供普通 HTTP，远程部署时应放在已认证的 TLS 反向代理之后。

服务实现 MCP `2025-11-25`：

| 方法 | 当前行为 |
| --- | --- |
| `initialize` | 公布工具能力，并协商表单询问支持 |
| `ping` | 返回空结果 |
| `tools/list` | 返回 `zeta` 和 `zeta-reply` |
| `tools/call` | 启动或继续一个 App Server Thread |
| `notifications/cancelled` | 请求中断相关 Turn |
| 服务端 `elicitation/create` | 把批准和用户输入请求转发给支持该能力的客户端 |
| `notifications/progress` | 投影数量受限且已脱敏的 Turn 生命周期更新 |

`zeta` 要求调用方生成 `invocationId` 并提供提示词。`zeta-reply` 要求新的 `invocationId`、同一
主体已获授权的 `threadId` 和提示词。两者都在 `structuredContent` 中返回稳定 Zeta 身份、
终态或阻塞状态，以及长度受限的内容。

## 2. Crate 边界

本 crate 拥有：

- MCP 分帧、初始化门控、工具模式和线协议校验；
- stdio 生命周期以及 Streamable HTTP 端点和会话生命周期；
- Bearer 认证、精确 Origin 校验和单进程连接数量限制；
- 请求取消与进度令牌关联；
- App Server 交互到 MCP 表单询问的投影；
- 按主体隔离的持久化调用回执，以及调用方到 Thread 的绑定；
- App Server 结果投影和输出截断。

本 crate 不拥有：

- Session、Thread、Turn 的状态迁移或存储；
- 模型、工具、策略、沙箱、凭据或工作区权威；
- 本地父子委托；
- App Server 协议定义；
- OAuth、租户配置、TLS 终止或远程 App Server 后端。

如果本 crate 直接打开产品存储、构造 `TurnExecutor`、调用供应商或写入 Thread 事件，就说明
架构所有权已经漂移。这些操作必须继续位于 App Server 客户端之后。

## 3. 模块与关键符号

| 模块/符号 | 职责 | 不得吸收 |
| --- | --- | --- |
| `lib.rs::{run_stdio,run_http}` | 校验选项，打开一个嵌入式 App Server 宿主和回执存储 | 协议分发或 Agent 状态 |
| `options.rs::{McpServerOptions,HttpServerOptions}` | 宿主拥有的根目录、运行时限制和 HTTP 安全配置 | 调用方权限覆盖 |
| `server.rs::McpServer` | 初始化门控、JSON-RPC 分发和活动调用取消 | Session/Thread 业务逻辑 |
| `server/events.rs::McpAgentEvents` | 进度和询问的线协议投影 | 策略决策 |
| `http.rs` / `http/wire.rs` | 已认证端点、MCP 会话和 SSE 分帧 | 持久化 Agent 权威 |
| `protocol.rs` | 线协议数据结构、工具模式和输入限制 | 直接透传 App Server 数据结构 |
| `agent.rs::AgentService` | 窄且可测试的 Agent 执行边界 | 模型/工具实现 |
| `agent.rs::AppServerAgentService` | 把启动/回复映射到类型化 App Server 调用和精确 Turn 更新 | 直接访问 Core/存储 |
| `agent/progress.rs` | 脱敏并限制供 MCP 进度使用的 Thread 更新 | 完整会话记录投影 |
| `interaction.rs` | 类型化批准/用户输入和 MCP 表单询问映射 | 自动批准 |
| `receipt.rs::ReceiptStore` | 按主体隔离的重放、单航班执行和 Thread 授权 | 产品会话记录存储 |
| `agent/outcome.rs` | 终态/等待中 Turn 投影 | Thread 修改或 MCP 分帧 |

当前调用路径：

```text
run_stdio 或 run_http
→ open_in_process_app_server
→ 创建按主体隔离的 AppServerAgentService
→ McpServer::handle_message
→ tools/call
→ session/create 或已授权的 thread/read
→ 启动时调用 session/thread/create
→ thread/subscribe + turn/start
→ 有界 thread/read 轮询和通知排空
→ progress 和可选 elicitation/create
→ 接受后调用 turn/interaction/resolve
→ 有界 MCP CallToolResult
```

每个 HTTP MCP 会话获得独立的 App Server 连接，但共享同一个嵌入式 App Server 宿主和回执权威。

## 4. 校验与限制

- `invocationId`：1–128 个 ASCII 字母、数字、`.`、`_` 或 `-`；
- 提示词：非空，最大 256 KiB；
- 默认 Turn 超时：60 秒；
- 调用方可请求的最大 Turn 超时：10 分钟；
- 轮询间隔：10 毫秒；
- MCP 工具结果内容：最大 256 KiB，包含截断标记；
- 进度：每次调用最多 256 条生命周期通知，并移除连续重复项；
- HTTP 请求体：最大 1 MiB；标头：最大 32 KiB；
- 默认最大 HTTP 连接数：64；
- 每个 MCP 会话最多保留 1024 个提前取消身份。

服务不通过工具参数接受工作区路径、原始配置映射、秘密、开发者指令或沙箱覆盖。工作区和执行
权威由宿主固定。

## 5. 身份、恢复与继续

`invocationId` 与 MCP JSON-RPC 请求 ID 以及所有 Zeta 产品身份互相独立。适配器为 Session、
Thread、Turn、交互解决和取消派生按主体命名空间隔离的稳定 App Server 命令 ID。

回执以原子方式持久化到 `<ZETA_STATE_ROOT>/mcp-server/receipts-v1.json`，并按主体隔离：

- stdio 使用本地用户主体；
- HTTP 从 Bearer 令牌派生不可逆主体标识；
- 参数相同的已完成调用会重放已保存结果；
- 同一身份配不同参数返回冲突；
- 并发重复调用返回执行中；
- 进程失败留下的运行中调用在重启后重新进入相同的确定性 App Server 命令，不分配新产品身份；
- `zeta-reply` 只接受持久绑定到同一主体的 Thread。

回执文件是适配器恢复索引，不是权威 Agent 状态。Session、Thread、Turn 状态仍在 App Server
存储中。一个状态根目录当前只能由一个 MCP Server 进程使用；跨进程文件锁和分布式回执存储
尚未实现。

等待交互的结果保持可恢复，不会被封存为已完成。使用相同调用身份重试时，即使进程重启，也能
继续精确的未完成交互。

## 6. 进度、交互与失败

调用方提供 `_meta.progressToken` 时，服务以同一精确令牌发出单调递增的
`notifications/progress`。消息只描述数量受限的生命周期状态，不暴露推理、提示词、工具参数
与结果、凭据或环境数据。

只有初始化后的客户端声明支持表单询问时，批准和用户输入请求才映射为 MCP
`elicitation/create`。接受的响应会转换回精确的类型化 App Server 请求身份，并通过
`turn/interaction/resolve` 发送。拒绝、取消或客户端不支持时返回阻塞的工具结果，绝不自动
批准。看起来在索要凭据或其他敏感值的用户输入问题不会通过表单询问发送。动态工具定义的交互
类型尚未投影。

长调用期间 stdio 继续读取，因此 `notifications/cancelled` 可以中断精确 Turn。stdio EOF
会取消活动工作，因为进程拥有该连接。HTTP/SSE 写入失败不能证明持久化 Turn 已取消；显式
MCP 取消仍是权威。客户端取消的请求不会返回 JSON-RPC 响应。

取消有两秒宽限期。服务端截止时间到达后，如果 Turn 没有进入权威终态，结果为
`outcomeUnknown`。无效 JSON-RPC 或协议请求使用 JSON-RPC 错误；工具参数、App Server 和
Agent 结果失败使用 `CallToolResult.isError`。

## 7. Streamable HTTP 安全性与生命周期

HTTP 端点：

- 接受 MCP 消息的 `POST`，返回 JSON 或 SSE；
- 对通知和客户端响应返回 `202 Accepted`；
- 初始化成功后使用安全随机 `MCP-Session-Id`；
- 后续请求必须提供 `MCP-Protocol-Version: 2025-11-25`；
- 支持通过 `DELETE` 终止会话；
- 独立 `GET` 流返回 `405`；
- 提供 Origin 标头时，必须精确匹配已配置值；
- 比较 Bearer 凭据时不使用提前退出的字符串比较。

HTTP 会话状态有意保持为进程本地状态。服务重启后，客户端重新初始化 MCP 会话，并以相同的
持久化 `invocationId` 重试。独立 GET SSE 流、`Last-Event-ID` 重投递、OAuth、多租户工作区
绑定和内置 TLS 均未实现。

## 8. 测试与扩展点

相邻单元和集成测试覆盖协议校验、进度令牌、询问身份、真实 App Server 启动/回复/进度、持久化
回执重开、HTTP 认证/Origin/会话/协议/SSE、取消和截断。`tests/stdio.rs` 启动真实二进制，
验证实时进度、重启重放以及重启后的 `zeta-reply`。

```text
cargo test -p zeta-mcp-server
cargo clippy -p zeta-mcp-server --all-targets -- -D warnings
```

当前扩展点与限制：

- 仍使用同步 App Server 请求和有界 Thread 轮询/排空；计划中的自有异步
  `AppServerSession` 将提供通用独立事件驱动；
- 不支持资源、提示词、根目录、采样或 MCP task 能力；
- 超大输出没有产物引用；
- 不支持动态工具交互投影；
- 不支持 HTTP 事件重放、OAuth/租户控制面、内置 TLS 或远程 App Server 后端；
- 没有原生远程 Agent 到 `DelegationId` 的桥接。

修改工具模式、限制、状态映射、回执身份、HTTP 安全或继续授权时，必须同步更新样例、测试、
本 README 和 [`docs/mcp-server.md`](../../docs/mcp-server.md)。
