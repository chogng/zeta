# zeta-app-server

## 核心模型

App Server 直接暴露 [`zeta-protocol` 定义的 canonical 产品模型](../../docs/protocol.md)，
不在服务层重新定义 Session、Thread、Turn 或 ThreadItem。

App Server connection lifecycle 不是产品 Session。App Server 只把 RPC
request/notification 机械映射为 canonical intent/update；它不执行 reducer、不推断 Thread
状态，也不拥有 Session/Thread persistence。

Session-owned Thread create/fork/archive 由 `SessionCoordinator` 编排；Turn start、interrupt 与
outstanding interaction resolve 由 `ThreadController` 提交，Turn 内的 model/tool loop 由
`TurnExecutor` 编排。对外只发布 `session/update` 与 `thread/update`，客户端通过
`subscribe(afterSequence)` 获得 snapshot 与 durable gap。

本地启动通过 `zeta-rollout::RolloutRepository` 打开 state root 并恢复
`SessionCoordinator`，因此 App Server 不自行重建文件布局、writer lease 或 Session/Thread
恢复顺序。
