# zeta-agent-session

> 本文拥有 `zeta-agent-session` 的实现与接入契约；跨 crate 的迁移状态由 [`app/docs/app-migration-plan.md`](../docs/app-migration-plan.md) 维护。

`zeta-agent-session` 是 app 侧 Agent Session 运行时的唯一所有者。它持有 App Server 会话、worker、订阅、文件与 Git 请求、语言服务器配置请求、命令队列和断线恢复；产品宿主只提供连接目标和事件投递函数。

## 边界

| 责任 | 所有者 |
| --- | --- |
| App Server client、事件流、Session/Thread 订阅 | `zeta-agent-session` |
| 文件读取与保存冲突检查、Git 快照与分支切换、语言服务器配置请求 | `zeta-agent-session` |
| 有界命令队列、worker 生命周期、远端重连窗口与断线期拒绝 | `zeta-agent-session` |
| Local/Remote 连接参数和实际连接建立 | 实现 `AgentSessionTarget` 的产品宿主 |
| 窗口事件投递、Session Tab、Composer、文件编辑器和 Workspace pane 状态 | `app` 产品组合层 |

本 crate 不依赖 `app` package、`zui` 或任何窗口类型。若 worker、协议订阅、文件/Git 请求重新出现在 `app/src/features/agent/agent_session.rs`，说明所有权已经漂移。

## 文件与关键接口

| 文件或接口 | 职责 |
| --- | --- |
| `src/lib.rs` / `AgentSession` | 启动和停止 worker，并把产品方法转换为有界队列命令 |
| `src/contract.rs` / `AgentSessionCommand`、`AgentSessionEvent` | 定义宿主与 worker 之间的 typed contract，以及断线期命令拒绝规则 |
| `src/worker.rs` / `run_agent_session`、`run_with_recovery`、`drive` | 持有连接、消费命令和 App Server 通知，并执行 Local/Remote 生命周期 |
| `src/worker/operations.rs` | 实现 Session/Thread 订阅、稳定文件快照、保存校验、Git 和配置请求 |
| `AgentSessionTarget` | 由宿主实现；保持连接类型并支持切换工作区后重新建立 App Server 会话 |

## 执行路径

1. 产品调用 `AgentSession::spawn`，传入 `AgentSessionTarget` 和 `AgentSessionEvent` sink。
2. `run_connection` 完成初始化、模型与命令目录读取、配置/Git 快照读取，并确保当前工作区存在活动 Session 与 Thread。
3. `drive` 轮询有界命令队列和 App Server 事件流；请求结果通过一次性 response channel 返回，durable 更新通过 event sink 投给产品 reducer。
4. Local 工作区切换使用同一目标的 `retarget` 创建新连接。Remote transport 失败进入最多 30 秒的指数退避恢复；断线期间的请求不会重放。

`SessionRequest::StartTurn` 始终使用当前协议的 `tool_mode` 字段。Thread 测试快照必须显式包含 `goal`，Session Thread projection 必须包含 `transcript`；不得在本 crate 恢复旧协议字段或添加兼容兜底。

## 失败语义

- `ClientError::Transport` 会被标记为连接丢失；Remote 目标允许重连，Local 目标直接结束 worker。
- worker 不可用时，`AgentSession` 在入队前返回 `AGENT_UNAVAILABLE_COMMAND_ERROR`。
- Remote 断线期间已排队的 request/response 命令会完成为同一错误，普通 fire-and-forget 命令只报告错误，不重放。
- 文件保存先比较读取时的 `TextFileDiskVersion`，版本变化或只读文件会明确失败。
- Git 不可用的协议错误会投影为 `GitSnapshot(None)`；其他 Git 错误不会被吞掉。

## 验证

- `cargo test -p zeta-agent-session` 覆盖队列容量、重连退避、断线期拒绝和订阅快照协议构造。
- `cargo check -p app --all-targets` 验证产品连接适配、事件 sink 和全部 app 测试构造点。
- 修改 App Server Session 协议时，同时检查 `src/worker/operations.rs`、`src/worker/operations_tests.rs` 和 app 中构造 `Thread`、`ConfigReadResult` 的测试。
