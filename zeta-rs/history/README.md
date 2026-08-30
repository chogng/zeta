# `zeta-history`

- 定义持久化 `ThreadEvent` 的稳定 envelope、event identity、时间戳、schema version 与 command receipt；不负责 I/O、事务或恢复。
- Thread event stream 是对话事实源；`session_id` 保存在 Thread 根事件中，用于聚合会话树，不存在独立 Session envelope。
- Store 校验与追加属于 `zeta-thread-store`，SQLite 映射属于 `zeta-state`，reducer 与恢复属于 `zeta-core`。
