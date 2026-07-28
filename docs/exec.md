# `zeta-exec` 架构与演进方案

> 目标物理位置：`zeta-rs/exec/`  
> 当前状态：Proposed；现有同名 crate 仍是底层 process executor，必须先迁移职责  
> App Server Client：[`app-server-client.md`](app-server-client.md)  
> App Server contract：[`zeta-app-server-api.md`](zeta-app-server-api.md)  
> Canonical 产品模型：[`protocol.md`](protocol.md)

## 1. 结论

`zeta-exec` 的长期角色是无交互 Agent runner。它把一次本地 CLI 请求或远程调度 Job 转换为
canonical Session/Thread/Turn command，持续消费 App Server result/update，并输出机器可处理的
终态、事件和退出状态。

它不是 shell command executor，也不是第二个 Agent runtime：

```text
local CLI / remote scheduler
            │
            ▼
        zeta-exec
            │ typed App Server request/update
            ▼
 zeta-app-server-client
            ▼
      zeta-app-server
            ▼
         zeta-core
            │ tool execution port
            ▼
 zeta-tool-executor / future zeta-exec-server
```

长期必须区分：

- `zeta-exec`：运行完整的 headless Agent Job；
- `zeta-tool-executor`：在本机执行一个经过 approval/sandbox 的 process；
- `zeta-exec-server`：未来把 process/filesystem execution 暴露给远程环境；
- scheduler protocol：提交、租约、取消和观察远程 Agent Job。

这四者不能共享一个含义含糊的 `exec` API。

## 2. Codex 参考与 Zeta 取舍

本方案参考本地 `../codex` workspace 的三层边界：

- `codex-rs/exec/` 是非交互 Agent 产品入口；
- `codex-rs/app-server-client/` 被 exec 与 TUI 共享，集中处理 embedded App Server startup、
  initialize、typed request/event channel、backpressure 和 bounded shutdown；
- `codex-rs/exec-server/` 单独负责 process、PTY、filesystem 和远程 execution environment。

Zeta 应采用：

- exec 与 TUI 共用 App Server Client；
- embedded hot path 使用 typed channel；
- request completion 与 server event stream 分离；
- cloneable request handle；
- bounded queue、lag signal 与显式 shutdown；
- headless exec 只通过 App Server 创建/恢复 Thread、启动/中断 Turn；
- remote Agent scheduling 与 remote process execution 分层。

Zeta 不照搬：

- exec 直接依赖 Core、rollout 或 provider 私有类型；
- caller 自由选择任意 `request_typed<T>` result 类型；
- 一个数千行 client/exec module；
- remote event 使用 unbounded queue；
- 复制一套与 canonical ThreadItem 平行的内部领域状态机；
- 按具体 notification 名字硬编码永久的 lossless/best-effort 列表。

Zeta 已有 durable aggregate sequence、snapshot + gap subscribe 和 typed command replay，应利用
这些契约实现 resync，而不是依赖“所有实时 event 永不丢失”。

## 3. 当前仓库偏差与迁移

当前 `zeta-rs/exec/` 定义的是：

- `CommandRequest`；
- `ExecutionLimits`；
- `ToolExecutor`；
- process stdout/stderr capture；
- timeout、sandbox 与 approval。

这些是底层 tool process execution，不是 headless Agent runner。
Sandbox 的共享 policy、macOS backend、Linux Bubblewrap 和 Windows restricted-token crate
边界见 [`sandboxing.md`](sandboxing.md)。

目标迁移：

```text
current zeta-rs/exec
  → zeta-rs/tool-executor
  → crate zeta-tool-executor

new zeta-rs/exec
  → crate/binary zeta-exec
  → headless Agent runner
```

若后续需要远程 process/filesystem execution：

```text
zeta-tool-executor
  → local backend of zeta-exec-server

zeta-exec-server-protocol
  → process / PTY / filesystem / HTTP execution contract
```

不能直接把当前 `ToolExecutor` 扩张为远程 Agent scheduler，否则 scheduler job、Agent lifecycle、
process lifecycle 和 sandbox authority 会混在同一 crate。

## 4. 职责与非职责

### 4.1 `zeta-exec` 拥有

- 一次 headless run 的输入、工作目录和输出选项；
- create/resume/fork 哪种产品入口意图；
- 使用 `zeta-app-server-client` 启动或连接 App Server；
- 创建/选择 Session 与 Thread，启动一个 Turn；
- 关联当前 Job、Session、Thread 与 Turn；
- 同时等待 request completion、App Server event、cancel 和 shutdown；
- 把 canonical update 映射为 human/JSONL/scheduler event；
- headless approval 行为；
- Ctrl-C 与 scheduler cancellation 到 `turn/interrupt` 的映射；
- terminal Turn status 到 `ExecOutcome`、退出码和 scheduler status 的映射；
- run-once 与长期 worker 的应用级生命周期。

### 4.2 `zeta-exec` 不拥有

- Session/Thread/Turn reducer；
- model loop、tool loop 或 context assembly；
- command receipt、writer lease 或 rollout recovery；
- App Server initialize、channel wiring 和 connection shutdown 的具体实现；
- process spawn、PTY、filesystem sandbox 或 network proxy；
- scheduler 的全局 queue、placement、quota 或 billing authority；
- 从日志、stderr 或人类文本推断 Turn 终态；
- 直接读取 Core、store 或 rollout 作为 App Server 旁路。

## 5. 本地 run-once

第一阶段支持一次进程运行一个 Agent Job：

```text
parse ExecRunRequest
  → start embedded AppServerSession
  → create/read Session
  → create/read Thread
  → subscribe Thread
  → start Turn
  → consume result + AppServerEvents
  → observe terminal durable update
  → unsubscribe
  → shutdown AppServerSession
  → emit ExecOutcome
```

建议状态机：

```text
Preparing
  → StartingAppServer
  → StartingThread
  → Running
  → Cancelling
  → Terminal
  → Closing
  → Closed
```

任意阶段失败都必须产生明确 `ExecOutcome`。Connection EOF 不是 Turn failure；若终态未知，
exec 应重新连接/read/subscribe 或返回 `OutcomeUnknown`，不能伪造 `Failed`。

## 6. Public API

目标 library API 以 typed request 和 sink 为边界：

```rust
pub struct ExecRunner;

impl ExecRunner {
    pub async fn run(
        request: ExecRunRequest,
        output: impl ExecEventSink,
    ) -> Result<ExecOutcome, ExecError>;
}
```

输入使用 enum，而不是多个 bool：

```rust
pub struct ExecRunRequest {
    pub entry: ExecEntry,
    pub output: ExecOutputMode,
    pub approval: HeadlessApprovalMode,
    pub app_server: AppServerTarget,
}

pub enum ExecEntry {
    New { title: String, input: Vec<InputItem> },
    Resume { session_id: SessionId, thread_id: ThreadId, input: Vec<InputItem> },
    Fork { session_id: SessionId, parent_thread_id: ThreadId, input: Vec<InputItem> },
}

pub enum AppServerTarget {
    Embedded(EmbeddedAppServerOptions),
    Remote(RemoteAppServerOptions),
}
```

`RemoteAppServerOptions` 表示连接相同 App Server contract，不表示 scheduler job protocol，也不
表示 remote process executor。

`zeta-cli` 负责 clap 参数和帮助；`zeta-exec` 负责这些参数解析后的运行语义。

## 7. 输出契约

Human 输出可以演进；JSONL 与 scheduler event 是机器契约，必须 versioned、typed 且与 stdout
诊断隔离。

```rust
pub enum ExecEvent {
    RunStarted {
        run_id: ExecRunId,
        origin: ExecOrigin,
        session_id: SessionId,
        thread_id: ThreadId,
    },
    TurnStarted { turn_id: TurnId },
    ItemStarted { item: ThreadItem },
    ItemUpdated { item_id: ItemId, update: ExecItemUpdate },
    ItemCompleted { item: ThreadItem },
    Warning { warning: ExecWarning },
    TurnCompleted { outcome: ExecTurnOutcome },
}

pub enum ExecOrigin {
    Local,
    Scheduled { job_id: JobId, attempt_id: AttemptId },
}
```

`ExecEvent` 是外部展示/automation envelope，不是新的 authoritative domain model：

- ID、status、item payload 优先机械复用 protocol；
- 展示所需的 delta 可以映射，但不能改变 durable 事实；
- terminal event 必须由 canonical Turn terminal status 产生；
- JSONL stdout 每行一个完整 event；
- stderr 只用于诊断；
- human 模式默认只把最终 Agent message 写入 stdout，进度写 stderr；
- event schema 变更必须有 fixture 与 compatibility policy。

## 8. Headless approval 与 server request

无交互运行不能等待不存在的用户界面。

```rust
pub enum HeadlessApprovalMode {
    DenyInteractiveRequests,
    UseConfiguredPolicy,
    Delegate(RemoteReviewerId),
}
```

规则：

- 没有 reviewer channel 时，approval/user-input server request 必须立即稳定拒绝；
- remote scheduler 支持 reviewer 时，请求必须携带 Job/Attempt/Thread/Turn/Tool identity；
- reviewer response 必须防重放并校验 action digest；
- approval timeout、scheduler disconnect 和 worker shutdown 都必须结束 pending server request；
- `--dangerously-*` 一类入口必须显式命名，不能成为远程 worker 默认值。

因此 App Server Client 的长期 event stream 需要同时支持：

- `ServerNotification`；
- `ServerRequest`；
- `Lagged/Desynced`；
- `ConnectionClosed`。

## 9. 远程调度

### 9.1 两种运行模式

```text
RunOnce
  = 一个 zeta-exec process
  = 一个 App Server session
  = 一个前台 Agent Job

Worker
  = 一个长期 zeta-exec worker
  = 一个或多个隔离的 App Server session
  = 多个有 lease 的 Agent Job
```

Worker mode 不能简单地在循环中调用 CLI `main`。它需要明确：

- worker registration；
- capability/host/workspace advertisement；
- job lease 与续租；
- attempt fencing；
- cancellation；
- event ack/cursor；
- reconnect 与 replay；
- draining shutdown；
- tenant、credential、workspace 和 state-root isolation。

### 9.2 Identity

以下身份不能混用：

| Identity | Owner | 用途 |
| --- | --- | --- |
| `ExecRunId` | zeta-exec | 当前 runner 内的一次运行 |
| `JobId` | scheduler | 一次逻辑远程任务 |
| `AttemptId` | scheduler | 一次 placement/execution attempt |
| lease/fencing token | scheduler | 阻止过期 worker 继续提交 |
| `CommandId` | App Server caller | durable command 幂等与 replay |
| Session/Thread/Turn ID | App Server domain | canonical Agent lifecycle |
| JSON-RPC request ID | App Server Client | 当前 connection 的 result pairing |
| durable sequence | aggregate | update replay 与 resync |
| scheduler event cursor | scheduler protocol | scheduler-side delivery ack |

推荐保存稳定映射：

```text
JobId + AttemptId
  ↔ SessionId + ThreadId + TurnId
  + last durable sequence
  + scheduler event cursor
```

Scheduler receipt 不是 App Server command receipt。两者可以在同一 Job flow 中关联，但不得放进
同一个 ID 或假设原子跨系统事务。

### 9.3 Remote Job flow

```text
scheduler assigns Job(attempt, lease)
  → worker validates isolation/capability
  → zeta-exec starts/selects AppServerSession
  → stable CommandId creates/resumes Session/Thread/Turn
  → worker streams mapped ExecEvent
  → scheduler acks event cursor
  → worker observes terminal durable update
  → worker commits terminal Job result with fencing token
```

断线恢复时：

- scheduler 可以重新投递同一 `JobId` 的新 `AttemptId`；
- worker 使用 mapping 与 App Server read/subscribe 判断原 Turn 是否仍运行或已终态；
- 相同逻辑 App Server command 重试复用原 `CommandId + exact payload`；
- durable gap 通过 snapshot + committed gap 恢复；
- transient delta 丢失只影响低延迟展示，不能影响 terminal Job result；
- 过期 lease 的 worker 不得提交 scheduler terminal result。

## 10. Remote Agent scheduling 与 remote execution plane

远程调度完整 Agent Job：

```text
scheduler → zeta-exec worker → App Server → Core
```

远程执行某个 tool process/filesystem operation：

```text
Core tool port → zeta-exec-server client → remote zeta-exec-server
```

二者的 security 和 lifecycle 不同：

| Plane | Authority | Stable unit | Disconnect behavior |
| --- | --- | --- | --- |
| Agent scheduling | scheduler + App Server | Job/Session/Thread/Turn | durable recovery/reschedule |
| Process execution | executor connection | process/filesystem request | terminate, resume or report unknown by exec protocol |

`zeta-exec-server` 不得接受 `turn/start`，`zeta-exec` scheduler adapter 不得提供裸
`process/start` 旁路。

## 11. Worker isolation 与并发

第一版 remote worker 应保守选择：

- 一个 Job 一个 workspace lease；
- 一个 tenant/credential scope 一个 App Server authority；
- 同一 Thread 只有一个 active writer；
- worker 只在 capability、sandbox 和 state-root scope 完全相容时复用 App Server session；
- 不在不同用户间共享 Resource ownership、subscription 或 transient cursor；
- worker restart 依赖 durable store 恢复，不依赖内存 pending table。

并发可以随 App Server per-Thread serialization 演进，但 scheduler placement 不能把“不同
Thread 可并行”误解为“不同 tenant 可以无隔离共享进程”。

## 12. Backpressure 与重放

所有 queue 有界：

- App Server Client request channel；
- App Server event channel；
- exec output sink；
- remote scheduler outbound buffer。

处理规则：

- durable update 不静默丢弃；
- event queue 满时发出 `Lagged/Desynced` 并按 aggregate sequence resubscribe；
- transient delta 可以按明确 policy 丢弃或合并；
- terminal update 在成功交付或完成 snapshot resync 前不能宣称 Job 完成；
- scheduler event 使用独立 cursor/ack，不能复用 Thread durable sequence；
- scheduler 长时间不 ack 时施加 backpressure、持久化 spool 或终止 attempt，不能无限占用内存。

## 13. 取消与关闭

RunOnce Ctrl-C：

```text
signal
  → turn/interrupt with stable CommandId
  → wait terminal update within deadline
  → unsubscribe
  → AppServerSession.shutdown()
  → interrupted ExecOutcome
```

Worker Job cancellation：

```text
scheduler cancel(JobId, AttemptId, fencing token)
  → validate current attempt
  → turn/interrupt
  → publish terminal Job event
  → keep worker/App Server alive for other jobs
```

Worker shutdown 应有具名模式：

```rust
pub enum WorkerShutdownMode {
    Drain,
    InterruptActiveJobs,
}
```

`Drop` 只做 best-effort cleanup；正常路径必须等待 job terminal handling 与 App Server Client
shutdown。

## 14. 目标 crate 与依赖

```text
zeta-cli
├─► zeta-tui ─────┐
└─► zeta-exec ────┤
                  ▼
       zeta-app-server-client
                  ▼
          zeta-app-server
                  ▼
              zeta-core
                  │
                  ▼
        zeta-shell-command
                  ▼
        zeta-tool-executor
```

未来可增加：

```text
zeta-exec-server-protocol
zeta-exec-server
zeta-scheduler-protocol
```

`zeta-exec` 可依赖 app-server-client、app-server-protocol、protocol 和窄 CLI/output utility。
它不依赖 Core、store、rollout、model provider、sandboxing 或 tool executor。

## 15. 实施阶段

### Phase 1：消除命名冲突

- 将当前 process `zeta-exec` 迁移为 `zeta-tool-executor`；
- 更新 shell-command、file-system、text-search、apply-patch、skills、plugins、MCP 文档与依赖；
- 为新 headless `zeta-exec` 保留 crate/binary 名称。

### Phase 2：本地 headless vertical slice

- 完成异步 App Server Client；
- 实现 new/resume、Turn start/interrupt；
- human/JSONL output；
- terminal status 与退出码；
- 显式 shutdown。

### Phase 3：可靠 automation

- 稳定 ExecEvent schema；
- output schema、last-message file、stdin 与 workspace input；
- approval delegation；
- reconnect/read/subscribe resync；
- integration fixtures。

### Phase 4：远程 worker

- scheduler protocol；
- Job/Attempt/lease/fencing；
- worker registration、heartbeat 和 draining；
- event cursor/ack 与 reconnect；
- tenant/workspace isolation。

### Phase 5：远程 execution environment

- 独立 exec-server protocol；
- process/PTY/filesystem handlers；
- authenticated transport；
- 与 Agent scheduler 分开的 reliability 与 security model。

## 16. 验证要求

- exec 与 TUI 使用同一个 App Server Client startup/connection contract；
- exec 不直接依赖 Core、store、provider、sandbox 或 tool executor；
- start/resume/interrupt 只通过 typed App Server API；
- request result 与 notification 可并行消费；
- JSONL stdout 永远是合法机器事件；
- terminal outcome 只来自 canonical Turn status；
- connection 断开不会伪造 Turn failure；
- Ctrl-C 和 remote cancel 都发送 typed `turn/interrupt`；
- headless approval 不会永久等待不存在的 UI；
- durable gap 可通过 read/subscribe 恢复；
- Job/Attempt/Command/request/sequence identity 有独立 contract tests；
- 过期 lease 无法提交 scheduler terminal result；
- worker shutdown 不泄漏 Job、App Server connection 或后台 task；
- remote Agent scheduling 与 remote process execution 没有协议旁路。
