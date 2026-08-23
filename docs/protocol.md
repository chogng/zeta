# 产品协议

> 物理位置：`zeta-rs/protocol/`  
> 当前状态：Session-first canonical contract 与 Agent interaction delivery/deadline 已落地
> Crate 类型、helper 与同步修改关系：[`zeta-rs/protocol/README.md`](../zeta-rs/protocol/README.md)
> 上层接口：[Zeta App Server API](zeta-app-server-api.md)  
> 执行计划：[Zeta Agent 执行架构与演进方案](zeta-agent-runtime-architecture.md)

> 文档所有权：本文件拥有跨组件语义、当前审计与演进方向；crate README 拥有当前源码接口、
> serialization invariant 与修改路径。

## 快速理解

协议层定义 Zeta 各组件共同使用的对象、意图、事实和更新；它让不同进程说同一种语言，但不执行
业务逻辑，也不选择存储或传输。

| 概念 | 回答的问题 | 不包含 |
| --- | --- | --- |
| Session、Thread、Turn、ThreadItem | 系统中的产品对象是什么？ | actor、锁、任务和数据库 |
| Command | 调用方想改变什么？ | 实际状态迁移和副作用 |
| Event | 哪个事实已经可靠提交？ | 文件格式和写盘策略 |
| Update | 消费方应该观察到什么变化？ | UI 呈现和重连策略 |
| 请求与响应 | Turn 正在等待什么外部输入？ | 批准界面和工具实现 |
| 稳定 ID、序列与游标 | 对象是谁、状态到第几版、从哪里继续？ | 传输 request ID 和连接状态 |

## 1. 结论

`zeta-protocol` 是 Zeta 跨 crate、跨进程和跨客户端共享的语义词汇表。它定义“系统中的对象、
意图、事实和更新是什么”，但不决定“由谁、何时、通过什么 I/O 执行”。

它的长期职责固定为：

- canonical `Session → Thread → Turn → ThreadItem` 产品模型；
- 稳定 ID 与共享 value object；
- `SessionCommand` / `ThreadCommand` 产品意图；
- `SessionEvent` / `ThreadEvent` durable fact；
- `SessionUpdate` / `ThreadUpdate` consumer-facing update；
- provider-independent model、tool、input、request/response 和稳定错误值。

它不得拥有：

- actor、mailbox、channel、task、锁和进程内执行实例；
- reducer、状态迁移策略、重试、超时、取消执行和恢复流程；
- 文件、数据库、JSONL、rollout framing、checksum 和 schema migration；
- JSON-RPC method、request ID、connection state 和 transport；
- provider HTTP payload、endpoint、credential 和 client；
- tool execution、sandbox、approval policy 和副作用。

一句话判断边界：

> 至少两个独立组件需要以相同语义理解的纯数据契约，可以进入 protocol；只服务于一个执行层
> 或带有 I/O、策略和生命周期行为的类型，不进入 protocol。

## 2. 所有权与依赖方向

`zeta-protocol` 不依赖任何 Zeta 内部 crate，目前只依赖序列化和 schema 生成库：

```text
serde / serde_json / schemars / ts-rs
                   │
                   ▼
             zeta-protocol
                   ▲
       ┌───────────┼────────────┬──────────────┐
       │           │            │              │
   zeta-core  history/store  zeta-api   app-server-protocol
       ▲                        ▲              ▲
       │                        │              │
  agent/execution          providers      clients
```

各消费者的准确职责：

| 消费者 | 使用 protocol 的方式 | 不得反向放入 protocol |
| --- | --- | --- |
| `zeta-core` | reducer 输入、projection、execution intent | actor message、policy、effect、recovery |
| `zeta-session-store` | 持久化 `SessionEvent` | sequence、timestamp、receipt envelope |
| `zeta-history` | 为 `ThreadEvent` 定义 persisted record envelope | Store、SQLite、reducer、UI history |
| `zeta-thread-store` | 完整读取和追加 `zeta_history::StoredEvent` | record serde shape、SQLite、reducer、UI Turn page |
| `zeta-api` | canonical model value 与 provider wire 互转 | HTTP DTO、header、endpoint |
| `zeta-model-provider` | model catalog 与 invocation request/response | transport、credential、retry |
| `zeta-app-server-protocol` | 机械复用 canonical view/update | JSON-RPC params、result、method registry |
| Desktop / CLI / TUI | 通过生成契约或 client 消费 snapshot/update | 客户端权威状态机 |

禁止出现：

```text
zeta-protocol → zeta-core
zeta-protocol → zeta-storage
zeta-protocol → zeta-app-server-protocol
zeta-protocol → provider implementation
```

## 3. Canonical 产品模型

唯一产品层级是：

```text
Session
└─ Thread
   └─ Turn
      └─ ThreadItem
```

### 3.1 Session

`Session` 是任务与 Thread 拓扑的 readable model，拥有：

- task title 和 lifecycle；
- Thread membership；
- root/fork lineage；
- Session 自己的 durable sequence。

Session 不嵌入 Thread 的 Turn/Item 历史。`ThreadOrigin::Fork` 保存
`parentThreadId + parentSequence`，表达不可变的分支历史锚点。

当前 canonical `ThreadOrigin` 包含 `Root`、`Fork`、`Rewind` 与 `AgentSpawn`。Agent spawn 使用
独立 `DelegationId`、不可变 `AgentContextSeed` 和跨 Thread message/result events；不复用
`Fork`。当前实现边界与尚未完成的 join/cancellation 语义见
[`core-multi-agent.md`](core-multi-agent.md)。

长期若 Session shared defaults 需要被多个客户端读取、修改并持久恢复，应以明确的
`SessionSettings` value 和 typed event 进入本契约；当前代码尚未形成该闭环，不能把 Core
配置快照直接塞进 `Session`。

### 3.2 Thread

`Thread` 是一个独立执行分支，拥有：

- immutable `SessionId`；
- Thread title 和 lifecycle；
- ordered Turns；
- Thread 自己的 durable sequence。

Session sequence 与 Thread sequence 是两个 aggregate 的 revision，不是 ID，也不是同一条
全局计数器。

### 3.3 Turn

`Turn` 表示一次用户意图驱动的 Agent 执行，不等同于一条聊天消息。当前状态词汇已经包含：

- `Created`、`Running`；
- `WaitingForApproval`、`WaitingForUserInput`、`WaitingForCapability`；
- `Cancelling`；
- `Completed`、`Failed`、`Interrupted`。

其中 `WaitingForUserInput` 与 `WaitingForCapability` 已有 durable interaction event、Core reducer
和 recovery 支持；它们不能再被解释成单纯预留 enum。App Server 已按 initialize capability +
Session-owned Thread subscription 选择 ephemeral owner，通过 `agent/request` 主动投递，并执行
按 interaction 语义决定的断连处理和 durable deadline cancellation；这些 runtime state 仍不属于
canonical Thread。

### 3.4 ThreadItem

`ThreadItem` 是 Turn transcript 的 durable 业务单元，当前包含：

- User Message；
- User Image；
- Agent Message；
- Reasoning；
- Plan；
- Tool Call；
- Tool Result。

Item 必须拥有稳定 `ItemId` 并归属一个 `TurnId`。Token delta 不是 durable Item；最终完成的
Item 才能进入 authoritative Thread history。

## 4. 四类契约必须分离

### 4.1 Command：请求改变状态

```text
SessionCommand
  Create / CreateThread / ForkThread / ArchiveThread / Complete / Archive

ThreadCommand
  StartTurn / InterruptTurn / ResolveUserInput / ResolveDynamicTool
```

Command 表达产品意图，不代表已经发生，也不能直接写入 projection。

`CommandId` 用于 retry-safe typed identity。`expectedSequence` 用于 optimistic
concurrency。JSON-RPC request ID 不得替代 CommandId。

前端关闭 Tab 不会扩展这组 canonical command。它是 App Server 的产品动作：外部通过
`session/request` 的 `Stop` operation 映射到 Core 的内部停止编排，持久化既有的 Session archive fact，并协调
中断其 child Thread 中的活动 Turn。这样 UI 的 Tab/session 语义不会泄漏进 `zeta-protocol`；连接
断开也不会被解释成产品 Session 停止。

所有 durable side effect 的 receipt 必须保存：

```text
CommandId
+ exact typed command payload
+ response sequence / stable result
```

重放规则固定为：

- 相同 `CommandId`、相同 typed payload：返回原结果；
- 相同 `CommandId`、不同 typed payload：返回 `CommandConflict`；
- 首次执行：校验 `expectedSequence`，并将 receipt 与首个业务 Event 原子提交。

重放时先匹配 receipt，再返回稳定结果，因此响应丢失后的安全重试不受 aggregate 后续 sequence
前进影响。不得用 JSON-RPC method + serialized params、sidecar idempotency ledger 或裸 request
ID 复制这套语义。

`SessionCommandEnvelope`、`ThreadCommandEnvelope` 曾经只在 protocol 内定义，没有第二个
执行层或 transport 消费者，已删除。Core command receipt 与 App Server params 各自属于其
拥有的执行/transport 层；将来只有确有至少两个消费者的共享 command wrapper 才能重新进入
protocol。

### 4.2 Event：已经 durable 的事实

`SessionEvent` 与 `ThreadEvent` 是 reducer 和 store 唯一接受的领域事实。

```text
Command
  → Core validate/policy
  → build Event
  → Store append
  → reducer
  → publish committed Update
```

Event 不携带 storage sequence、timestamp、schema version、event ID 或 command receipt。
Session stored envelope 由 `zeta-session-store` 定义；Thread persisted record 由
`zeta-history` 定义。Core 构造 record，Store 只校验并提交。

必须保持：

- Event 使用过去式事实命名；
- Event 足以从空状态重建 canonical projection；
- transient delta、delivery ack、RPC request 和 provider response 不进入 Event；
- reducer policy 不编码进 Event helper。

委托执行有两个 durable fact。`ThreadEvent::TurnExecutionAttempted` 在 backend 跨越外部副作用边界
前写入 Turn；恢复时已有 attempt 的 in-flight Turn 结果未知，禁止重放。成功完成后，
`ThreadEvent::TurnExecutionBound` 将 Thread 不可变地绑定到
`{ backend, remoteThreadId, executionScope }`，并投影为
`ThreadSnapshot.turn_execution_binding`。它不保存 access token、远端 Turn ID、pending request 或
stream cursor；execution scope 是不含路径和凭据的 opaque Workspace authority identity，同一 Thread
不能跨 scope 或改绑到另一个 backend/remote thread。

### 4.3 更新：面向消费者的变化

`SessionUpdate` / `ThreadUpdate` 服务于 UI、CLI、TUI 和订阅客户端。

`Committed { event }` 携带 durable fact；`ItemStarted`、`ItemDelta`、`PlanUpdated` 是可丢失的
低延迟更新。客户端必须能仅依赖 snapshot + durable committed gap 恢复正确状态。

```text
durableSequence
  = aggregate 已提交到第几个事实

StreamCursor { streamInstanceId, sequence }
  = 某次 transient stream emitter incarnation 中瞬态消息的位置
```

两者不能合并。进程重启后 stream cursor 可以失效，durable sequence 不能失效。

当前瞬态 update、Core streaming model loop，以及 OpenAI Responses、OpenAI-compatible Chat
Completions、Anthropic Messages 的生产 HTTP/SSE 路径已经贯通。三种 endpoint 都使用各自原生
wire stream；其他 SSE profile、NDJSON 与 WebSocket 仍需按真实协议逐项接入。

### 4.4 Agent 请求/响应：Turn 中的双向等待

`TurnInteraction` 是一个 Turn 当前未完成交互的 durable value，拥有 `RequestId`、可选 `ItemId`、
typed `AgentRequest` 与可选绝对 deadline。Readable `Turn` 只公开不含 payload 的
`PendingInteraction`（request ID、kind、optional item/deadline），避免普通 Thread read 被误用为
delivery channel。`AgentRequestEnvelope` 在完整 `TurnInteraction` 基础上补齐
`SessionId + ThreadId + TurnId`，供 App Server 构造 delivery；`AgentResponseEnvelope` 只做
response correlation。

它们用于关联：

- structured user input；
- dynamic tool call；
- 将来明确接受的 capability interaction。

它们不是 ThreadCommand 或 JSON-RPC envelope。对应的 durable Thread facts 是：

```text
InteractionRequested { interaction }
InteractionResolved { requestId, response }
InteractionCancelled { requestId, reason }
```

`InteractionRequested` 将 Running Turn 转为对应 waiting status，resolved/cancelled 关闭同一个
pending interaction并返回 Running。App Server 的 canonical `SessionRequest::ResolveInteraction`
是 client 发起的 retry-safe aggregate command，必须带 exact `RequestId`；它不是 Agent request
自身。

完整生命周期必须明确：

```text
request fact committed
→ owner connection selected (App Server ephemeral state)
→ delivered
→ response / timeout / cancellation / disconnect
→ durable Turn transition
```

这里的 `owner connection` 不能进入 `ThreadEvent`、`TurnInteraction` 或 canonical Thread snapshot：
connection ID 在断开后失效，不是产品身份。App Server 必须维护短暂的 `RequestId → connection`
delivery assignment；断开时，尚未产生外部副作用的 approval/user-input 可以重新选择并重投递，
已经投递的 dynamic tool 必须追加 `InteractionCancelled { reason: OwnerDisconnected }` 并按
unknown outcome 收口，不能交给另一连接重试。任何分支都不得让 durable Turn 永远停在 waiting。
deadline 同样是 durable absolute instant，但 timer 和超时 policy 属于执行控制层。

当前已完成 canonical contract、Core reducer/recovery、`session/request::ResolveInteraction`、
initialize capability、connection owner selection、`agent/request` 主动 delivery、按 interaction
语义处理 owner 断连与 deadline timer。具体 host 只声明它实际支持的 interaction kind；对于
dynamic tool，还必须在 `dynamicTools` 中声明它实际承载的 exact tool name，仅声明
`DynamicTool` kind 不会获得其他动态工具的执行权。

## 5. Sequence、Cursor 与 ID

### 5.1 ID 回答“是谁”

当前主要 ID：

| ID | 身份范围 |
| --- | --- |
| `SessionId` | 一个产品任务 |
| `ThreadId` | 一个独立执行分支 |
| `TurnId` | 一次 Agent 执行 |
| `ItemId` | 一个 transcript item |
| `ToolCallId` | 一次工具调用 |
| `CommandId` | 一个 retry-safe command |
| `RequestId` | 一个双向 Agent request |
| `StreamInstanceId` | 一次 transient stream emitter incarnation |

### 5.2 Sequence 回答“第几个状态版本”

- Session sequence 只排序 membership、lineage 和 Session lifecycle；
- Thread sequence 只排序 Turn、Item 和执行终态；
- stream cursor 只排序同一 stream incarnation 的 transient update；
- fork parent sequence 是历史锚点。

任何 ID 都不能由 sequence 推导，任何 sequence 也不能充当 ID。

Session 与 Thread 使用独立 sequence，是因为它们是不同的冲突域：

| Sequence | 排序内容 | 并发边界 |
| --- | --- | --- |
| Session | membership、fork lineage、Session lifecycle | 同一 Session 的拓扑修改 |
| Thread | Turn、Item、工具与执行终态 | 同一 Thread 的执行修改 |

如果合成一条 sequence，子 Thread 的执行会无意义地争用 Session revision，多分支并行会产生
虚假的 optimistic-concurrency 冲突，单个 Thread 的恢复也会被迫读取整个 Session 的历史。
逻辑 sequence 独立不代表物理实现重复；不同 aggregate 可以复用同一个 event-stream engine，
只保留各自的 typed validation、reducer 和 writer scope。

### 5.3 当前校验不一致

`SessionId`、`ThreadId`、`TurnId`、`ItemId`、`CommandId`、`RequestId`、`StreamInstanceId`、
`ToolCallId`、`ProviderId`、`ModelId` 与 `ToolName` 都使用 fallible constructor 与 validated
deserialize。JSON Schema 的最小长度只是外部提示，不能替代 Rust 侧校验。

长期需要统一：

- 外部输入反序列化必须拒绝空 ID；
- 内部 ID constructor 不应制造无效值；
- ID 格式只规定必要约束，不把当前生成算法写入 protocol；
- 不使用裸 `String` 表达已经拥有专用 ID/newtype 的概念。

## 6. 供应商无关的模型契约

`model/invocation.rs` 定义 model invocation 的 canonical value：

- `ModelRequest`；
- input message / content / tool result；
- tool definition / tool choice；
- reasoning config；
- `ModelResponse` / response item；
- token usage / stop reason。

`zeta-api` 负责把它转换为 OpenAI、Anthropic、Qwen 等 provider wire。Provider 特有字段不能
反向污染 canonical model。

`model/catalog.rs` 定义 model catalog metadata：

- provider/model identity；
- access kind（API key、subscription、local、enterprise 或 unknown）；
- context window；
- tools、reasoning、parallel tool call、personality capability；
- reasoning effort；
- auto-compaction threshold metadata；
- model availability、catalog freshness、lifecycle 与 metadata quality 的跨 crate 值。

目录 cache、scope、refresh、generation、字段 provenance 和 typed resolution 属于
[`zeta-models-manager`](../zeta-rs/models-manager/README.md)，不进入纯 protocol value 层。App Server 的
`model/list` DTO 从后端静态目录统一投影模型 identity、display name、access kind、context window、
automatic compaction threshold、capabilities、reasoning efforts 和默认 personality。Models Manager
内部的 discovery availability 不进入产品模型列表，也不作为 Session 选择或发送消息的门禁；调用失败
由对应 Turn 的稳定错误承载。
Zeta 产品内置模型的具体 rows 只在 `zeta-model-provider-config::STATIC_MODEL_CATALOG` 维护，protocol
只提供 rows 使用的通用 value types，不拥有另一份模型枚举。

这里的 compaction 仅是声明信息。Context builder、token accounting policy、summary
checkpoint 和 compaction 执行都不属于 protocol。

当前需要继续收敛的重复表示：

| 概念 | 当前表示 | 长期处理 |
| --- | --- | --- |
| Tool name | `ToolName` | 已用于 durable Item、dynamic tool 和 model contract |
| Tool arguments | `String`、`serde_json::Value` 并存 | 按 durable canonical 与 provider value 分层 |
| Tool call ID | `ToolCallId` | 已用于 durable Item、dynamic tool 和 model contract |
| Model response provenance | 无真实消费者 | 已删除 speculative `ResponseItemId` 与 provider raw response ID；有可证明的跨 provider 需求后再引入 |
| Usage | model response 有 usage，Thread history 未建模 | 先明确 billing/diagnostic/product ownership |

## 7. Config 值的边界

`Theme`、`Personality`、`ApprovalMode`、`SandboxMode`、`WebSearchMode` 可以留在 protocol，
前提是它们确实被多个组件以同一语义使用。

以下内容属于 `zeta-config`，不能进入 protocol：

- 文件位置与读取；
- defaults、layer merge 和 precedence；
- provider registry resolution；
- credential reference resolution；
- 更新事务与 authority snapshot。

`Patch<T>` 只表达 missing/null/value 三态。字段是否允许 clear、如何持久化、冲突如何处理，
仍由拥有该配置的应用层决定。

## 8. 目录结构与公开 API

### 8.1 当前已实现结构

目录已经按 aggregate-first 迁移；每个实现模块仍保持在约 200 LoC 以下，因此 `item` 与
`model` 暂时只拆到已有的稳定边界：

```text
src/
├── ids/{session,thread,command,interaction,tool}.rs
├── session/{model,status,origin,command,event,update}.rs
├── thread/{model,status,command,event,update}.rs
├── turn/{model,status}.rs
├── item/{mod,plan}.rs
├── interaction/{envelope,user_input,request_user_input,dynamic_tool}.rs
├── model/{invocation,catalog}.rs
├── config/{values,patch}.rs
├── stream.rs
├── tool_name.rs
├── error.rs
├── contract_tests.rs
└── lib.rs
```

模块保持 private，`lib.rs` 已使用 named exports 暴露唯一的 crate API；新增内部类型不会被
glob re-export 意外变成 public API。

`ids/` 不再按模糊的 `product`、`operation` 分类；文件名必须直接表达 identity 的所属范围：

- `session.rs` 只拥有 Session identity；
- `thread.rs` 拥有 Thread 层级中的 Thread、Turn、Item identity；
- `command.rs`、`interaction.rs`、`tool.rs` 分别拥有 retry command、双向 interaction 和 tool
  execution 的 correlation identity；
- stream emitter identity 与 `StreamCursor` 同属一个 cursor contract，因此定义在 `stream.rs`，
  不在 `ids/` 中伪装成通用执行 identity。

这种拆法比按“产品/操作”分组更稳定，因为后两者不是 protocol 中可审计的领域对象，也混淆了
`CommandId`、`RequestId` 和 `ToolCallId` 互不相同的生命周期。

### 8.2 长期目标结构

长期按领域所有权组织，而不是把所有 aggregate 的 command、event 或 update 各放进一个横向
大模块：

```text
zeta-rs/protocol/
├── Cargo.toml
└── src/
    ├── lib.rs                         # private modules + named public exports
    ├── contract_tests.rs              # crate-level serde/schema contract tests
    ├── ids/
    │   ├── mod.rs
    │   ├── session_id.rs              # SessionId
    │   ├── thread_id.rs               # ThreadId
    │   ├── turn_id.rs                 # TurnId
    │   ├── item_id.rs                 # ItemId
    │   ├── command_id.rs              # CommandId
    │   ├── request_id.rs              # RequestId
    │   └── tool_call_id.rs            # ToolCallId
    ├── session/
    │   ├── mod.rs
    │   ├── model.rs                   # Session / SessionThread
    │   ├── status.rs                  # SessionStatus / SessionThreadStatus
    │   ├── origin.rs                  # ThreadOrigin
    │   ├── settings.rs                # 仅放已接受的 shared semantic defaults
    │   ├── command.rs                 # SessionCommand
    │   ├── event.rs                   # SessionEvent
    │   └── update.rs                  # SessionUpdate + envelope
    ├── thread/
    │   ├── mod.rs
    │   ├── model.rs                   # Thread
    │   ├── status.rs                  # ThreadStatus
    │   ├── command.rs                 # ThreadCommand
    │   ├── event.rs                   # ThreadEvent
    │   └── update.rs                  # ThreadUpdate / ItemDelta + envelope
    ├── turn/
    │   ├── mod.rs
    │   ├── model.rs                   # Turn
    │   └── status.rs                  # TurnStatus
    ├── item/
    │   ├── mod.rs                     # ThreadItem public sum type
    │   ├── message.rs
    │   ├── reasoning.rs
    │   ├── plan.rs
    │   └── tool.rs
    ├── interaction/
    │   ├── mod.rs
    │   ├── envelope.rs                # AgentRequest / AgentResponse correlation
    │   ├── user_input.rs
    │   ├── request_user_input.rs
    │   └── dynamic_tool.rs
    ├── model/
    │   ├── mod.rs                     # provider-independent model contract
    │   ├── request.rs
    │   ├── response.rs
    │   ├── content.rs
    │   ├── tool.rs
    │   ├── reasoning.rs
    │   └── catalog.rs                 # ModelRef / ModelInfo / capabilities
    ├── config/
    │   ├── mod.rs
    │   ├── values.rs                  # shared semantic config values only
    │   └── patch.rs
    ├── stream.rs                      # StreamInstanceId + StreamCursor，不与 durable sequence 混放
    ├── tool_name.rs                   # 跨 item/model/interaction 的 validated value
    └── error.rs
```

这里选择 aggregate-first，而不是顶层 `command/`、`event/`、`update/`，原因是：

- Session 与 Thread 是独立 aggregate，各自的 model、intent、fact 和 consumer update 应共同
  演进；
- 修改一个 aggregate 时，其 serde shape、状态词汇和 contract tests 可以放在同一目录审查；
- 避免未来形成同时承载 Session、Thread 以及其他 aggregate 的巨型横向模块；
- `turn` 和 `item` 是 Thread 下的共享产品概念，但也被客户端、trace 和 provider-neutral
  context 使用，因此保留独立顶层模块。

目标树表达所有权，不要求立即创建所有空文件。模块只有在已有类型或真实 vertical slice 需要
时才建立；小型 payload 在拆分能提高内聚前可继续留在父模块。

### 8.3 已完成的文件迁移

| 当前文件 | 目标位置 |
| --- | --- |
| `ids.rs`、`session_id.rs`、`thread_id.rs` | `ids/` |
| `response_item_id.rs` | 删除；没有真实 provenance 消费者 |
| `session.rs` | `session/model.rs`、`session/status.rs`、`session/origin.rs` |
| `thread.rs`、`turn.rs` | `thread/`、`turn/` |
| `items.rs`、`plan_tool.rs` | `item/` |
| `command.rs` | 按 aggregate 拆到 `session/command.rs` 与 `thread/command.rs` |
| `session_event.rs`、`thread_event.rs` | 对应 aggregate 的 `event.rs` |
| `thread_update.rs` | 拆到 `session/update.rs`、`thread/update.rs` 与 `stream.rs` |
| `agent_request.rs`、`user_input.rs`、`request_user_input.rs`、`dynamic_tools.rs` | `interaction/` |
| `models.rs`、`zeta_models.rs` | `model/`；后者明确重命名为 `catalog.rs` |
| `config_types.rs`、`patch.rs` | `config/` |
| `tool_name.rs`、`error.rs` | 暂留明确的顶层共享值模块 |
| `protocol_tests.rs` | `contract_tests.rs`；细粒度测试随所属模块迁移 |

迁移保持所有模块 private，`lib.rs` 以 named exports 维持唯一公共 API。模块级测试放在实现
文件的 sibling `*_tests.rs` 中并用显式 `#[path = "..._tests.rs"]` 引入；跨模块 serde/schema
约束保留在 `contract_tests.rs`。开发阶段直接同步调用方，不增加旧 module alias 或兼容 re-export。

## 9. 当前完成度

| 能力 | 状态 | 客观判断 |
| --- | --- | --- |
| Session-first hierarchy | 已完成 | Core、stores、App Server 和客户端已经使用 |
| Session shared defaults | 未完成 | 尚无 canonical settings、typed event 与恢复闭环 |
| Session/Thread 独立 sequence | 已完成 | model、store 和 fork lineage 已覆盖 |
| durable Event 与 live Update 分离 | 基础完成 | store 类型只能接受 durable event |
| typed Session/Thread command | 基础完成 | Core receipt 已使用；无消费者的 shared envelope 已删除 |
| Tab 关闭到 Session 停止 | 已完成 | Chat 前端 → App Server `session/request` Stop → Core 内部停止编排；连接断开不触发停止 |
| ThreadItem durable transcript | 基础完成 | text/image message、reasoning、plan、tool item 可重建 |
| transient Item streaming | 类型已定义 | 当前没有完整异步 model stream producer |
| Agent request/response | 基础完成 | durable request/resolve/cancel、deadline value、request correlation 和 typed resolve 已实现；owner delivery/timer 未实现 |
| waiting Turn lifecycle | 基础完成 | event/reducer/recovery 已实现；异步 Agent loop 的继续执行尚未实现 |
| provider-independent model values | 基础完成 | provider adapters 已使用，tool name/call ID 已收敛 |
| stable error taxonomy | 早期基础 | 当前只有少量 Turn error code |
| usage/compaction provenance | 未完成 | 只有 model usage 与 threshold metadata |
| ID validation | 已完成 | constructor 与 deserialize 都拒绝空的 canonical ID |
| public API discipline | 已完成 | private modules、named exports 与 speculative envelope 清理已落地 |

## 10. 演进方案

### 阶段 P0：冻结职责边界

- 以本文作为 `zeta-protocol` crate 架构基线；
- 新类型必须说明至少两个真实消费者；
- review 阻止 execution、wire、storage 和 provider DTO 泄漏；
- contract tests 固定 Session/Thread/Event/Update 的关键 serde shape。

完成条件：新增 protocol 类型都能明确回答“谁拥有行为，谁只是共享语义”。

### 阶段 P1：统一身份与公共 API（完成）

- 产品 ID 已使用 validated construction/deserialization；
- 同一工具概念的裸 `String` 已迁移到 `ToolName` / `ToolCallId`；
- 没有真实消费者的 `ResponseItemId` 与 command envelope 已删除；
- aggregate-first 模块、private modules 和 named exports 已落地，未预建空模块。

完成条件：无效 ID 无法通过公开 constructor 或 deserialize 创建，公共 API 可被显式审计。

### 阶段 P2：补齐异步 Turn 交互契约（完成）

该阶段必须与 `zeta-agent`、Core 和 App Server 的 vertical slice 同步完成：

- 已明确 owner 是 App Server ephemeral delivery state，deadline/cancel/disconnect 的 durable
  boundary；
- 已增加 `InteractionRequested` / `InteractionResolved` / `InteractionCancelled` facts 与
  `TurnInteraction` snapshot；
- 已让 waiting Turn status 可由 durable event 重建，recovery 保留仍可行动的 wait；
- 已接通带 `RequestId` 的 `session/request::ResolveInteraction`；
- App Server 已把 request、initialize capability、owner selection、主动 delivery、deadline 与
  disconnect re-selection 组成真实的 Server → Client vertical slice；
- 保证 Agent request 与 Thread command 不混为一类。

完成条件已满足：进程等待响应时可恢复明确 waiting 状态；full request 只投递给 selected owner；
断连可重选；deadline 形成 durable cancellation + stable Turn failure。

### 阶段 P3：收敛模型/工具契约

- Context builder 只输出一套 canonical `ModelRequest`；
- provider adapter 不读取 ThreadItem 或 Core snapshot；
- tool definition/call/result 的 name、ID 和 arguments 表示统一；
- 明确 parallel tool call、refusal、reasoning 和 partial output 语义；
- 加入跨 provider contract fixtures。

完成条件：同一 canonical request 可通过不同 provider adapter，结果可无损回到 Agent loop
需要的共享语义。

### 阶段 P4：补齐流式处理、usage 与压缩来源

- 根据真实 streaming producer 增加最小必要 delta；
- 明确 transient update 的 stream cursor 和重连降级；
- 评审 usage 是否属于 durable product fact、diagnostic trace 或 billing projection；
- compaction checkpoint 必须引用其覆盖的原始历史范围；
- 不在 protocol 中实现 token policy 或 summary 算法。

完成条件：客户端丢失所有 transient update 后仍能从 snapshot + durable gap 恢复，compaction
不会破坏原始历史可追溯性。

### 阶段 P5：发布后的版本政策

当前开发阶段直接修改 canonical 类型并同步所有调用方，不保留 deprecated alias 或双写。

真正发布并需要读取用户长期数据后，再引入：

- 明确的 semantic compatibility policy；
- stored-event schema migration；
- App Server wire version negotiation；
- trace/export format version；
- breaking change 的离线迁移工具。

这些版本机制分别属于拥有其格式的 crate，不能全部塞入 `zeta-protocol`。

## 11. 变更规则

新增或修改一个 protocol 类型时，必须同时检查：

1. 它是 command、event、update、request/response，还是 readable model；
2. authority 属于 Session、Thread、执行控制、connection 还是 provider；
3. 是否 durable；如果 durable，是否足够重建 projection；
4. 是否需要 aggregate sequence、stream cursor 或独立 ID；
5. 是否包含 provider、transport、storage 或执行策略细节；
6. 至少两个真实消费者是谁；
7. Rust serde、JSON Schema、TypeScript 和 App Server DTO 是否需要同步；
8. 是否需要更新 reducer、store validator、fixtures 和客户端 projection。

禁止：

- 为未来可能需要而提前增加没有调用方的 enum variant；
- 使用 `String`、`bool`、含义模糊的 `Option` 代替已有强类型语义；
- 把 request、update 或 provider output 直接追加为 durable event；
- 让客户端根据文本、日志或 variant 名推断未声明状态；
- 用 schema derive 代替 Rust constructor/deserialize validation；
- 为兼容开发期旧代码保留 alias、旧 route 对应类型或隐式转换。

## 12. 验证门

每次修改 `zeta-protocol` 至少执行：

```bash
cargo fmt --manifest-path Cargo.toml --all -- --check
cargo clippy --manifest-path Cargo.toml -p zeta-protocol --all-targets -- -D warnings
cargo test --manifest-path Cargo.toml -p zeta-protocol
cargo test --manifest-path Cargo.toml -p zeta-app-server-protocol
```

如果改动进入 App Server external contract，还必须：

```bash
cargo run --manifest-path Cargo.toml \
  -p zeta-app-server-protocol --bin write_schema_fixtures
node desktop/scripts/sync-app-server-protocol.mjs
corepack pnpm --dir desktop run typecheck:renderer
```

关键 contract tests 必须覆盖：

- canonical hierarchy 与 fork lineage；
- durable event 不混入 transient wrapper；
- Session/Thread sequence 独立；
- durable sequence 与 stream cursor 独立；
- ID 的空值和非法值；
- Tool/Model canonical value 的 provider round-trip；
- Agent request correlation 与 response mismatch；
- generated schema/TypeScript fixture 一致性。

## 13. 验收标准

`zeta-protocol` 达到长期稳定状态时，应满足：

- 所有跨组件共享概念只有一个 canonical 名称和表示；
- Core、store、provider 和 App Server wire 的行为细节均未泄漏；
- durable fact 可以完整重建产品 projection；
- transient update 可全部丢失而不损坏权威状态；
- Session、Thread、执行实例和 request 的身份/顺序边界不会混淆；
- 所有公开 ID 和 value object 在构造与反序列化时保持不变量；
- bidirectional request 在 response、timeout、cancel、disconnect 和恢复上有完整契约；
- provider adapter 只做 canonical model 与 wire 的机械转换；
- public exports、serde shape、schema 与 fixtures 都可被自动验证；
- 不存在无调用方的 speculative contract 或开发期兼容层。
