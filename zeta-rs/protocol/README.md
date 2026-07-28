# `zeta-protocol`

> 本 README 解释 canonical data contract 的代码组织、关键 symbol 与变更约束。跨 Core、store、
> App Server 和客户端的语义方向见 [`docs/protocol.md`](../../docs/protocol.md)。

`zeta-protocol` 是 Zeta 跨 crate、跨进程和跨客户端共享的 provider-independent 语义契约。
它定义 ID、产品模型、command、durable event、consumer update、interaction 与 model invocation
values；它没有 I/O、reducer、store、actor、transport、provider wire codec 或 effect policy。

## Ownership rule

一个 type 只有在至少两个独立组件需要以相同语义理解它时才适合进入本 crate。执行层 private
message、store envelope、JSON-RPC params、provider payload 和 UI view state 不应为了“复用”进入
canonical protocol。

```text
serde / schemars / ts-rs
          │
          ▼
   zeta-protocol
      ▲    ▲    ▲
      │    │    └─ app-server-protocol / clients
      │    └────── zeta-api / model-provider
      └─────────── core / session-store / thread-store
```

`lib.rs` 保持所有实现 module private，并显式 export public contract。新增 module 不应直接公开，
避免调用方依赖内部文件布局。

## Public contract map

| Domain | 主要 symbols | 语义 |
| --- | --- | --- |
| Identity | `SessionId`, `ThreadId`, `TurnId`, `ItemId`, `CommandId`, `RequestId`, `ToolCallId` | non-empty typed string identity |
| Product model | `Session`, `SessionThread`, `Thread`, `Turn`, `ThreadItem` | `Session → Thread → Turn → Item` snapshot |
| Intent | `SessionCommand`, `ThreadCommand` | 请求改变状态，不表示已发生 |
| Durable fact | `SessionEvent`, `ThreadEvent`, `ToolExecutionAuthority` | reducer/store 接受的过去式事实 |
| Tool execution | `ProcessExecutionOutput`, `SandboxDenialOutput`, `ToolExecutionOutput`, `ToolReplaySafety` | executor、Core 与 durable audit 共享的原始结果/重放语义 |
| Consumer update | `SessionUpdateEnvelope`, `ThreadUpdateEnvelope`, `ThreadUpdate`, `ItemDelta` | durable committed 与 transient projection |
| Interaction | `TurnInteraction`, `AgentRequest`, `AgentResponse`, `PendingInteraction` | Turn 等待/恢复的 typed request-response |
| Approval | `ActionApprovalRequest`, `ActionApprovalResponse`, capability/decision enums | exact action/policy binding |
| Model catalog | `ProviderId`, `ModelId`, `ModelRef`, `ModelInfo`, `ModelCapabilities`, `ModelPreset` | provider-neutral selection metadata |
| Model invocation | `ModelRequest`, `InputItem`, `Message`, tools, `ModelResponse`, `ModelStreamEvent` | canonical model I/O |
| Config values | `Patch<T>`, `ApprovalMode`, `SandboxMode`, `Personality`, `Theme`, `WebSearchMode` | shared pure values |
| Stable failure | `StableTurnError`, `StableTurnErrorCode` | durable/client-safe Turn failure |

除 model invocation 中需要浮点 temperature 和 arbitrary JSON tool schema 的 types 外，公共值尽量
derive `Eq`、serde、`JsonSchema` 与 `TS`。是否生成 TypeScript/schema artifact 由
`zeta-app-server-protocol` 负责，不在这里写文件。

## 内部接口地图

本 crate 大部分代码是 public data shape，private interface 数量刻意很少。下面这些 helper 承载
关键 invariant：

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `identifier!` | private macro | 为 canonical IDs 生成 constructor、serde、schema、TS | 所有 ID deserialize 必须重用 constructor validation |
| `validate_identifier` | crate-private | 拒绝 empty/whitespace identity | 不引入 storage/provider-specific syntax |
| `model_identifier!` | private macro | 生成 `ProviderId`/`ModelId` contract | 保持与 canonical ID 相同的 non-empty invariant |
| `ThreadEvent::kind` | public method | stable internal event-kind label | 与 enum variants exhaustive 同步 |
| `ThreadEvent::thread_id` | public method | 提取 aggregate identity | 新 event 必须显式加入 match |
| `SessionEvent::session_id` | public method | 提取 aggregate identity | 新 event 必须显式加入 match |
| `ThreadItem::{item_id,turn_id}` | public methods | 提取 item ownership | 所有 item variants 必须有两种 typed ID |
| `AgentRequest::kind` / `AgentResponse::kind` | public methods | request-response kind correlation | request/response family 必须 lockstep |
| `TurnInteraction::pending_state` | public method | full durable request → redaction-safe snapshot metadata | broad snapshot 不得包含 request payload |
| `ModelInfo::effective_auto_compact_token_limit` | public method | min(configured, 90% known context) | `Unknown` context 不伪造 limit |
| `ModelResponse::{text,tool_calls}` | public methods | 读取 canonical output projection | 不丢弃 authoritative raw output vector |

## 四类状态契约

### Command

`SessionCommand` 与 `ThreadCommand` 表达 typed intent：

```text
SessionCommand
├─ Create / Complete / Archive
└─ CreateThread / ForkThread / ArchiveThread

ThreadCommand
├─ StartTurn / InterruptTurn
└─ ResolveApproval / ResolveUserInput / ResolveDynamicTool
```

Command ID、expected sequence、receipt 和 idempotent replay 是 Core/store execution concern，不被
塞入每个 command enum。

### Event

`SessionEvent`/`ThreadEvent` 只表达已经 durable 的领域事实：

```text
Command
└─ Core validation + policy
   └─ Event
      ├─ store append envelope
      ├─ reducer
      └─ ThreadUpdate::Committed / SessionUpdate::Committed
```

Event 不携带 sequence、timestamp、schema version、event ID 或 transport metadata。特别地，
`ThreadEvent::ToolExecutionStarted` durable 地保存 action digest、policy revision 和
`ToolExecutionAuthority`，但不负责判断 authority 是否有效。一次 sandbox denial 获得新的 exact
authority 后，`ThreadEvent::ToolExecutionEscalated` 在重试前保存完整
`SandboxDenialOutput` 与新 authority；Core reducer 负责验证它只能引用 started、未完成且尚未
escalate 的 Tool Call。

### Update

`ThreadUpdate::Committed { event }` 是 durable；`ItemStarted`、`ItemDelta` 与 `PlanUpdated` 是可丢失
的低延迟 projection。`ThreadUpdateEnvelope` 同时区分：

- `durable_sequence`：aggregate 已提交事实的位置；
- `stream_cursor`：某个 transient stream instance 内的位置。

重启后 stream cursor 可以失效，durable sequence 不可以。客户端不能把 transient delta 当成最终
Thread history。

### Interaction

`TurnInteraction` 保存完整 durable request、request ID、optional item ID 与 absolute deadline。
`TurnInteraction::pending_state()` 只保留 request ID、kind、item ID 和 deadline，避免普通 snapshot
成为敏感 request delivery channel。

```text
ThreadEvent::InteractionRequested
└─ Turn.pending_interaction = PendingInteraction
   ├─ App Server 通过 AgentRequestEnvelope 定向投递 full interaction
   └─ typed AgentResponse
      └─ Resolve* ThreadCommand
         └─ InteractionResolved / InteractionCancelled
```

Connection ownership、delivery retry、deadline timer 与 disconnect policy不属于 protocol。

## Serialization contract

Domain enums 通常使用 internally tagged camelCase JSON：

```json
{
  "type": "turnStarted",
  "threadId": "thread_1",
  "turnId": "turn_1"
}
```

这不是实现细节：App Server schema、TypeScript binding、fixtures、stored events 和 external
clients 都可能依赖 field/variant spelling。Rename、retag、required/optional 改动都是 contract
change，不能只修 Rust compile error。

Typed IDs 的 `new` 和 serde deserialize 都拒绝 empty/whitespace values。不要为外部 payload
derive 一个绕过 constructor 的 deserialize path。

## Model contract

Model types是 provider-neutral semantic IR：

```text
ModelRequest
├─ instructions
├─ InputItem::{Message,ToolResult}
├─ ToolDefinition / ToolChoice
└─ reasoning / token / temperature settings

ModelResponse
├─ ResponseItem::{Text,Refusal,Reasoning,ToolCall}
├─ ModelUsage
└─ StopReason
```

`ModelStreamEvent` 当前只有 text/reasoning delta；最终 `ModelResponse` 仍是 authoritative outcome。
Provider endpoint、header、cache-control、JSON shape、SSE event name 和 retry hint 不进入本 crate。

`ToolName` 拒绝 empty 与 provider-specific slash syntax；provider adapter 负责 wire name conversion，
不应污染 canonical name。

## 同步修改关系

| 修改 | 必须同步检查 |
| --- | --- |
| ID type/invariant | `identifier!`、deserialize、schema/TS、store keys、contract tests |
| Command variant | Core handler/reducer plan、App Server RPC params、idempotency tests |
| Event variant/field | store validation、Core reducer/recovery、`kind`/ID extractor、updates、fixtures |
| Tool execution output/replay field | executor capture、Tool adapter、Core retry gate、durable escalation、schema/TS、contract tests |
| ThreadItem variant | item ID/turn ID extractors、Core projection、TUI/render、schema |
| Interaction family | request/response kind、pending redaction、Core resolve command、App Server routing |
| Model field/variant | `zeta-api` 三套 codecs、provider adapters、schema consumers |
| serde tag/name/optionality | TypeScript、JSON Schema、golden fixtures、backward compatibility decision |
| Stream cursor/update | broker replay/gap semantics、client subscription tests |

## 方向偏差检查

- Protocol type 包含 channel、lock、future、filesystem path reader 或 database handle：runtime/I/O 漂移；
- Event 包含 sequence/schema/checksum：store envelope 漂移进 domain；
- JSON-RPC method/request ID 进入 command：transport 漂移进 intent；
- Provider endpoint/header/payload 进入 model types：wire protocol 漂移；
- Reducer 或 approval decision method 进入 value types：policy/behavior 漂移；
- `PendingInteraction` 包含 full request payload：snapshot 绕过定向 delivery；
- Transient `ItemDelta` 被追加为 durable item：authoritative history 语义漂移；
- Consumer 新建平行 Session/Thread/model struct：canonical source 已分叉。

## 测试、限制与演进

```text
cargo test -p zeta-protocol
bazel test //zeta-rs/protocol:protocol-unit-tests
```

`contract_tests.rs` 当前验证 JSON shape、Session lineage、durable/transient cursor 分离、interaction
payload redaction、approval binding、structured sandbox escalation、UserInput variants、
ToolName/ID validation、tool identity 和 auto-compact limit。新增测试继续放在独立 sibling
test file。

当前 protocol 已覆盖 Session-first product contract 与 durable interaction 基础；multi-agent
delegation、shared session settings、完整 streaming tool-call delta 和更强 schema compatibility
治理仍是潜在演进。是否进入本 crate应先证明它是共享语义，而不是为了提前占位。
