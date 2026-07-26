# zeta-thread-store

`zeta-thread-store` 定义 Thread durable event stream 的 storage-neutral contract。它只接受
`zeta_protocol::ThreadEvent`，并负责 expected-sequence atomic append、typed envelope 和
schema version；它不保存 `ThreadUpdate`、token delta、actor state 或 JSON-RPC payload。
当前 stored-event schema 从 `ThreadCreated` 固定 immutable `SessionId`，command receipt
保存 exact typed `ThreadCommand`。

Session 结构历史由独立的 `zeta-session-store` 负责：

```text
SessionStore
    membership / lineage / settings / lifecycle

ThreadStore
    Turn / ThreadItem / request lifecycle
```

两者拥有独立逻辑 sequence，但 `zeta-storage` 的物理 framing、checksum、atomic append 和
断尾恢复共用一个 event-stream engine。跨 stream 的 Thread create/fork 由 Core 可恢复 saga
协调。`zeta-rollout` 只负责组合两个 typed store 与 lease 并恢复 runtime；`zeta-rollout-trace`
只读取这些 port 生成诊断/导出 artifact，二者都不改变 ThreadStore contract。
