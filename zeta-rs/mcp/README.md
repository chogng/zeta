# zeta-mcp

> 本 README 是 MCP 产品运行时当前实现的 canonical 文档。跨 crate 产品语义、授权、
> durability 与演进阶段见 [`../../docs/mcp.md`](../../docs/mcp.md)；单连接协议和 transport
> 实现见 [`../rmcp-client/README.md`](../rmcp-client/README.md)。

本 crate 把一组已经过宿主解析和授权的 MCP server definition 变成隔离 session、不可变工具
catalog、精确 binding 和可取消调用。它不读取 Config 或 Plugin，不解析 credential reference，
不审批工具调用，也不写 Core Thread。

## 边界与文件

| 文件 / symbol | 当前职责 |
| --- | --- |
| `definition.rs` / `McpServerDefinition` | 接收宿主已 materialize 的 stdio 或 Streamable HTTP transport |
| `session.rs` / `McpSession` | product runtime 使用的单 session 窄接口 |
| `session.rs` / `McpSessionFactory` | transport/session 创建扩展点；production 由 `RmcpSessionFactory` 实现 |
| `catalog.rs` / `McpCatalogSnapshot` | 保存跨 server、不可变、按 generation 冻结的工具视图 |
| `catalog.rs` / `McpToolBinding` | 保存 model alias、exact remote identity、definition digest 和两个 generation |
| `runtime.rs` / `McpRuntime` | 启动 server set、发现 catalog、路由调用、观察失效和有界 shutdown |
| `output.rs` / `project_tool_result` | 把不可信 RMCP result 投影到 `zeta-tools::ToolOutput` |
| `error.rs` | 区分构造失败、未开始、结果未知和无效结果 |

依赖方向固定为：

```text
zeta-mcp
  → zeta-rmcp-client
  → zeta-tools / zeta-protocol
  → zeta-config::McpServerId
```

`zeta-mcp` 不应依赖 `zeta-core`、App Server、stores、Plugin manager 或 secret store。若本 crate
开始读取用户配置、决定 approval 或追加 Thread event，即表示 ownership 已漂移。

## Public contract

宿主先完成 enablement、trust、可执行文件解析和 credential materialization，再构造
`McpServerDefinition`。`McpRuntime::start` 使用 production RMCP factory；
`start_with_factory` 允许测试或宿主注入隔离 session。

`McpStartupPolicy::RequireAll` 在任一 server 启动或 catalog 失败时关闭已启动 session 并整体失败；
`AllowPartial` 保留健康 server，并通过 `McpStartupDiagnostic` 返回脱敏诊断。重复 server ID 永远
拒绝。

每个 `McpToolBinding` 同时固定：

- model-visible `ToolName`；
- `McpServerId + exact remote tool name`；
- definition digest；
- connection generation；
- catalog generation。

`McpRuntime::call_tool` 不按 remote name 重新搜索。binding 不属于当前 snapshot 时在发送前返回
`NotStarted`，防止旧 catalog 劫持新连接。调用方应从 `catalog()` 获取 definition/binding，并在
Core safe point 以整个新 runtime 替换旧 runtime。

## 内部调用与校验

```text
McpRuntime::start[_with_factory]
→ reject_duplicate_servers
→ McpSessionFactory::connect
→ discover_server_tools
   → McpSession::list_tools
   → project_tool
   → zeta_tools::from_mcp_tool_projection
→ McpCatalogSnapshot::new

McpRuntime::call_tool
→ 校验 cancellation、snapshot binding 和 object arguments
→ McpSession::call_tool
→ project_tool_result
```

`discover_server_tools` 是 catalog 的关键 private owner：它限制 page 数、每 server tool 数和序列化
byte 数，拒绝空/重复 cursor 与重复 remote tool name。`exposed_name` 对
`server ID + NUL + exact remote name` 做 SHA-256，并生成带 12 个 hex 字符后缀的稳定合法 alias；
可读 slug 不是 identity。

`RuntimeClientHost` 拦截 `ToolListChanged` 并把该 server 标成 `Stale`，同时继续转发 progress、
server cancellation 和 elicitation。它不会原地修改 catalog；宿主必须在 safe point 重建
runtime。

`project_tool_result` 保留 text 和合法 `image/*` block，将其他 RMCP block 序列化为 text，并对
最终 text/data URL 按 UTF-8 byte 数执行 output limit。MCP `isError` 映射为
`ToolOutputStatus::Error`，不是 transport failure。

## 失败、取消与恢复

- 调用前 cancellation、非 object arguments 和 stale binding：`McpCallError::NotStarted`。
- 请求发送后的 cancellation、timeout、断线或协议错误：
  `McpCallError::OutcomeUncertain`；上层不得自动重放有副作用调用。
- 无法安全投影或超过 output limit：`McpCallError::InvalidResult`。
- `McpRuntime::shutdown(self)` 消费 runtime，逐 server shutdown 并返回失败诊断。

`McpSession` 实现必须将 cancellation 传入 transport；production adapter 使用
`RmcpClient::call_tool_with_cancellation` 发送 MCP cancellation notification。取消只停止本地等待
和尽力通知 server，不证明远端副作用未发生。

## 接入义务

App Server adapter 仍需负责：

- 从 `zeta-config` 和 Plugin snapshot 选择已启用 server；
- 通过 secret/process/network owner materialize definition；
- 把 `catalog().model_definitions()` 与 frozen binding 接入 Core `ToolService`；
- 执行 approval、durable Tool Call/Result、unknown-outcome recovery；
- 在 list-changed/config generation 后选择 safe point 重建 runtime；
- 把 progress、elicitation 和诊断交付给有界 interaction channel。

这些义务尚未在 App Server 接线，因此本 crate 已实现不等于终端用户已经可以调用外部 MCP。

## 测试与修改影响

```text
cargo test -p zeta-mcp
cargo clippy -p zeta-mcp --all-targets -- -D warnings
bazel test //zeta-rs/mcp:mcp-unit-tests
```

`catalog_tests.rs` 覆盖 alias 的合法性、边界和 exact identity；`runtime_tests.rs` 使用 fake
factory 覆盖多 server catalog、冻结 binding 路由、部分启动、失效通知和旧 generation 拒绝；
`output_tests.rs` 覆盖 result limit 与 remote error status。修改 alias/hash、limit、binding 字段、
startup policy、cancellation 或 output projection 时，必须同步检查这些测试、本 README 和
[`../../docs/mcp.md`](../../docs/mcp.md)。

## Current limitations / Extension points

- Current：tools 的多 server discovery/call 已实现；resources、prompts、roots、sampling 和
  tasks 尚未进入 product runtime。
- Current：list-changed 只标记 stale；自动 reconnect、backoff、health state machine 和 catalog
  replacement 由后续 host lifecycle 实现。
- Current：production factory 可连接 direct stdio 与 unauthenticated/bearer Streamable HTTP；
  sandboxed launcher、OAuth 和 credential refresh 需由宿主注入。
- Current：没有 App Server/Core adapter、approval 或 durable recovery。
- Extension point：实现 `McpSessionFactory` 可注入受 sandbox 管理的 process、远端执行或自定义
  HTTP stack，而不改变 catalog/binding 语义。
