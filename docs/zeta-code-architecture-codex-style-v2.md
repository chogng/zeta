# Zeta Code 架构收口记录

> 状态：Done。原计划已经并入当前架构；领域概念以 [`domain-model.md`](domain-model.md)，Core 以 [`core.md`](core.md)，协议以 [`protocol.md`](protocol.md) 为准。

## 已确定的边界

- Thread event stream 是对话唯一持久事实源；`session_id` 只是 Thread tree 的共同分组身份。
- Session 是按 `session_id` 聚合的只读树视图，不建立 SessionCoordinator、Session store、Session event 或第二套 sequence。
- Project 是可选的长期组织关系；Environment 是执行位置；`cwd`、dirs 与 grants 是环境内的有效工作范围。
- Permission 表示动作种类，Grant 表示已授予范围，ApprovalRequest 表示授权交互，AuthorizationDecision 表示当前动作的 allow/deny。
- Workspace 只保留在编辑器窗口、多根 folder、配置作用域或 Cargo workspace 等确有该语义的地方。

## 当前调用链

```text
Product host
    │ JSON-RPC
    ▼
App Server
    │ typed Thread request
    ▼
ThreadController
    │ ThreadEvent batch
    ▼
ThreadStore
    │
    ▼
SQLite
```

树级读取通过 `session/read`、`session/list` 和 `session/subscribe` 提供便利；拓扑变化只发送
`session/changed` 刷新提示，具体 durable gap 仍由每个 Thread 的 update stream 承担。
