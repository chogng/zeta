# `zeta-rollout-trace`

> 本 README 解释 read-only trace capture。Durable source contract 见
> [`zeta-history`](../history/README.md)、[`zeta-session-store`](../session-store/README.md) 与
> [`zeta-thread-store`](../thread-store/README.md)；跨系统 durability/privacy 方向见
> [`docs/zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md)。

`zeta-rollout-trace` 从一个 Session topology stream 和它计划创建的 Thread streams 生成可序列化
诊断 artifact。Trace 保留每个 aggregate 的原 sequence，不制造虚假的 global ordering，也不能
成为 runtime authority。

## 接口地图

| Symbol | 可见性 | 职责 |
| --- | --- | --- |
| `capture_session_trace` | public function | 读取 Session，再按 `ThreadCreationPlanned` 读取 child Thread |
| `RolloutTrace` | public struct | format version、Session ID、Session events、Thread traces |
| `ThreadRolloutTrace` | public struct | 一个 planned Thread 的 ID 与原始 `zeta_history::StoredEvent` |
| `ROLLOUT_TRACE_FORMAT_VERSION` | public constant | self-contained trace artifact version，当前为 `1` |
| `RolloutTraceError` | public enum | Session missing 或 source store failure |
| `seen_thread_ids` | private local set | 保持首次计划顺序并去重重复 planned event |

```text
capture_session_trace(session_store, thread_store, session_id)
├─ SessionStore::load
├─ empty → SessionNotFound
├─ iterate stored Session events in source order
├─ select SessionEvent::ThreadCreationPlanned
├─ deduplicate ThreadId
├─ ThreadStore::load(each)
└─ RolloutTrace { independent sequences preserved }
```

一个 planned Thread 即使 events 为空也必须包含在 trace 中；它表示 saga 已 durable plan、但 child
stream 尚未创建。静默丢弃会抹去最需要诊断的中断状态。

方向偏差：

- 按 timestamp 合并 Session/Thread events：伪造 total order；
- 从 existing Thread directory 推断 membership：绕过 Session authority；
- reducer 后只导出 projection：丢失原始 recovery evidence；
- crate 提供默认文件上传：把含敏感内容的 artifact 变成隐式 exfiltration path；
- trace 被输入 runtime decision：diagnostic artifact 成为第二 authority。

## Privacy、错误与测试

Trace 可能含用户输入、Tool arguments/results 和 external identifiers。本 crate 只返回内存 value，
不写文件、不上传、不脱敏。调用方拥有 redaction、access control、retention 和 sharing policy。

`ThreadStore` error 包含 exact `thread_id` context；Session 空 history 映射为
`SessionNotFound`，不是一个合法空 trace。

```text
cargo test -p zeta-rollout-trace
bazel test //zeta-rs/rollout-trace:rollout-trace-unit-tests
```

修改 trace field 时 bump format version 并审查 downstream readers。当前没有 streaming capture、
filter/redaction policy、partial range 或 cross-Session export；这些未来能力不得改变 source
sequence 或 read-only 性质。
