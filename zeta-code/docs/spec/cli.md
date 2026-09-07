# CLI 命令与输出

- 本文维护终端命令的用户入口、输出和退出码；模块与连接职责见[CLI 架构](../design/cli.md)。
- 命令完整参数以 `zeta --help` 及对应子命令帮助为准；共享无交互执行规则见[exec](../../../docs/exec.md)。

## 命令入口

| 命令 | 用户结果 | 执行归属 |
| --- | --- | --- |
| `zeta` | 打开交互式 TUI | CLI 启动与连接，TUI 接收输入和显示 |
| `zeta ask` | 提交 prompt，输出最终 Agent 消息 | 创建 Session、Thread 后启动 Turn |
| `zeta exec` | new / resume / fork，Human 或 JSONL 输出 | 通过同一 Agent 与工具执行路径 |
| `zeta login` / `zeta config` | 登录或修改配置 | 类型化后端请求 |
| `zeta app-server` | 选择服务监听或连接入口 | 委托共享 server host |
| `zeta mcp-server` | 提供 MCP 服务入口 | 委托对应服务 |

### 使用示例

```bash
zeta
zeta ask "解释当前仓库"
zeta exec "检查测试失败"
zeta exec --jsonl --auto-review "检查测试失败"
zeta exec --resume SESSION_ID THREAD_ID "继续处理"
zeta exec --fork SESSION_ID PARENT_THREAD_ID --title "替代方案" "尝试另一种修复"
zeta login
zeta config
zeta app-server --listen stdio://  # compatibility alias to zeta-server-host
zeta app-server connect            # profile-scoped local App Server
zeta mcp-server
zeta mcp-server --listen http://127.0.0.1:8787/mcp

```

## 输入与输出

| 场景 | 契约 |
| --- | --- |
| `ask` / `exec` | 运行 Agent 任务，不作为任意 shell 命令执行器 |
| Human | stdout 默认只写最终 Agent 消息；进度和诊断写 stderr |
| `--jsonl` | 每行一个完整 `ExecEvent` 并立即 flush；stdout 不混入日志 |
| 机器事件 | 使用版本化结构，显式映射类型化通知；具体字段由 [exec 输出契约](../../../docs/exec.md#7-输出契约)维护 |
| 中断 | Ctrl+C 请求中断当前 Turn 并等待后端终态 |

## 退出码

| 代码 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 一般运行失败 |
| 2 | 参数错误，或无界面运行需要交互 |
| 75 | Turn 已启动，但无法确认最终结果 |
| 130 | 用户中断或 Turn 被中断 |

## 无交互审批

| 模式 | 行为 |
| --- | --- |
| 默认 deny | 拒绝需要交互的请求，不等待不存在的 UI |
| automatic review | 审查结论由后端策略决定 |
| 显式 bypass | 按用户显式选择和后端策略运行，CLI 不自行批准 |

- 远程委派审查仍是方案，不能当作已交付模式。
- 参数、非 TTY 处理和审批完整约定见 [exec](../../../docs/exec.md#8-无界面批准与服务端请求)。

## MCP 监听参数

| 参数 | 用途 |
| --- | --- |
| `stdio://` | 标准输入输出监听 |
| `http://IP:PORT/PATH` | HTTP 监听 |
| `ZETA_MCP_BEARER_TOKEN` | HTTP bearer |
| `ZETA_MCP_ALLOWED_ORIGIN` | 可选的精确 Origin |

非 loopback 部署必须由 TLS/auth reverse proxy 提供外部保护；OAuth 与租户管理不由 CLI 定义，详见[MCP server](../../../docs/mcp-server.md)。
