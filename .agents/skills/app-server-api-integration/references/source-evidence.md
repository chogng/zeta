# 源码证据

本文件只用于维护本 skill、核对依据或处理其他 reference 未覆盖的边界。路径采用两个中性根：前端参考源码根以下写作 `src/`，后端参考 crate 使用 `../app-server*`。这些路径证明职责与行为，不要求复制实现，也不表示目标仓库已具备对应文件。

## 前端 process、service 与 connection

| 路径 | 已验证结论 |
| --- | --- |
| `src/platform/agentHost/electron-browser/localAgentHostService.ts` | renderer 创建 protocol client，拥有 state/request traffic、initialize、pending、notification 与 reconnect；renderer 通过 nonce 取得 MessagePort。 |
| `src/platform/agentHost/browser/agentHostProtocolClient.ts` | 每个 client 拥有 request allocator、pending map、message classification、state subscription 与 close；这些职责不在 Main。 |
| `src/platform/agentHost/electron-main/electronAgentHostStarter.ts` | Main 启动一个 utility process；每个 renderer connection 请求调用一次 process `connect()`，再把独立 MessagePort 交给对应窗口。Main 不替 renderer 配对 protocol request。 |
| `src/workbench/services/agentHost/electron-browser/agentHostService.ts` | renderer Workbench service 在本地与远程 host client 之间选择并注册统一 service；产品 UI 依赖 service，不依赖 process starter。 |

由这些源码可得：前端参考架构是“一份 host process、多 renderer 独立 connection、renderer protocol client、领域 service”，不是“Main 一条 protocol connection、多窗口共享 pending”。文件、账户、配置、模型、skills、Project、Thread、process 等 API 应继续从各自领域 service 进入，只把原 TypeScript 后端调用替换成 app-server adapter。

## Thread UI 的条件证据

仅当 backend Thread 需要进入 Agents Window 时使用以下证据：

| 路径 | 已验证结论 |
| --- | --- |
| `src/sessions/SESSIONS.md` | Provider-neutral services 聚合 Provider；`ISession.workspace` 表示 Session 操作的 Workspace；Provider 拥有 backend state、resource URI、恢复和认证的适配。 |
| `src/sessions/services/sessions/common/sessionsProvider.ts` | `ISessionsProvider` 封装一个执行环境，拥有 Workspace discovery、Session creation/listing 和 picker contribution；一个 Provider 可服务多个 Session type。 |
| `src/sessions/services/sessions/common/session.ts` | canonical Session ID 由 `providerId + resourceUri` 的唯一 helper 生成；消费者比较 resource identity。 |
| `src/sessions/contrib/providers/agentHost/browser/baseAgentHostSessionsProvider.ts` | committed adapter 从 backend raw ID 构造 backend URI、provider resource 与 canonical Session ID；draft 提交后使用独立 replacement lifecycle 切换 committed facade。 |
| `src/sessions/contrib/providers/agentHost/AGENT_HOST_SESSIONS_PROVIDER.md` | Provider cache 拥有 facade identity；catalog event 表达 membership，observable 表达 mutable state。 |
| `src/platform/agentHost/AGENTS.md` | protocol-visible Session 可以包含主 Chat 与 peer Chat；provider conversation/thread 是 Chat backing，不等于 orchestrator Session。没有完整 Chat catalog 时不能假设 Session 与 provider conversation 是多 Chat 关系。 |

Sessions 是 Thread UI 的 adapter，不是 app-server 所有 API 的统一入口。

## 后端协议、Project 与 Environment

| 路径 | 已验证结论 |
| --- | --- |
| `../app-server/README.md` | 默认 stdio 是单路 JSONL；WebSocket 标记为 experimental/unsupported；Unix socket 使用 WebSocket upgrade。每条 connection 必须独立完成 `initialize` → `initialized`。 |
| `../app-server-transport/src/transport/stdio.rs` | stdio 直接读取进程 stdin 并写 stdout，只形成一条进程级 stream。 |
| `../app-server-transport/src/transport/mod.rs` | transport 使用有界队列，产生 opened/closed/message，并保持 connection identity。 |
| `../app-server-protocol/src/protocol/common.rs` | 一个注册点绑定 typed client request、response、server notification、server request 和 serialization scope；Project 与 Environment method 当前标记为 experimental。 |
| `../app-server-protocol/src/protocol/v2/project.rs` | Project 是后端对象，拥有 ID、name、roots、metadata、position 与时间；协议提供 list/read/create/import/update/move/delete 和 changed notification。 |
| `../app-server-protocol/src/protocol/v2/thread.rs` | `thread/start` 可携带 `projectId` 与 `environments`；Project assignment 和 Environment selection 是独立字段。 |
| `../app-server-protocol/src/protocol/v2/thread_data.rs` | Thread 的 canonical `projectId` 明确由 app-server 拥有；Thread 同时保存 cwd、status、Turn 与其他 durable metadata。 |
| `../app-server-protocol/src/protocol/v2/environment.rs` | Environment 用独立 `environmentId` 管理 add/info/status；它不包含 Project identity。 |
| `../app-server/src/request_processors/projects.rs` | app-server 执行 Project CRUD、发送 Project changed 与 Thread Project updated notification；前端不是 Project persistence owner。 |
| `../app-server/src/request_processors/thread_processor.rs` | Thread start/update 校验 Project 是否存在，并把 assignment 写入 Thread store；fork/restore 使用后端 Thread identity。 |
| `../app-server/src/message_processor.rs` | initialize 是 per-connection 门禁；其他 request 在初始化前被拒绝；统一 processor 做 typed dispatch。 |
| `../app-server/src/request_serialization.rs` | serialization key 由协议 scope 产生；connection-scoped resource key 包含 connection ID，不同 key 并行。 |
| `../app-server/src/outgoing_message.rs` | outgoing owner 统一发送 response/error/notification，维护 server request callback，并按 connection 路由。 |
| `../app-server/src/fs_watch.rs` | 资源以 `(connection_id, watch_id)` 隔离；重复 ID 被拒绝；unwatch 等待任务结束；connection close 清理资源。 |

Project 不是 Session Host，也不只是 Workspace。Project 是后端 catalog object；Workspace 只在 Thread UI 场景表达目录范围；Environment 是 Thread/Turn 执行目标。Provider 仅封装 Agents Window 的执行边界。

## 生成协议与运行时校验

| 路径 | 已验证结论 |
| --- | --- |
| `../app-server-protocol/src/export.rs` 与 `../app-server-protocol/src/precomputed_exports.rs` | TypeScript 与 JSON Schema 由协议 crate 生成，生成物不可手改。 |
| `../app-server-protocol/schema/typescript/ClientRequest.ts` | 当前生成物提供 method/params request union，但没有 method → params → response map。 |
| `../app-server-protocol/schema/typescript/ServerRequest.ts` | server request union 包含 Thread/Turn 交互 request，也包含无 Thread identity 的 account/attestation request。 |
| `../app-server-protocol/schema/typescript/ServerNotificationEnvelope.ts` | notification 有生成 union/envelope，但 TypeScript type 本身不校验 transport 的 `unknown` frame。 |
| `../app-server-protocol/schema/typescript/` | 当前没有完整运行时 decoder 或等价生成校验器。 |
| `../app-server-protocol/src/rpc.rs` | 错误 envelope 包含 numeric code、message 和可选结构化 data；稳定领域分类不能只依赖 message。 |

## 当前阻塞

- **正式多 connection 本地 transport**：前端参考拓扑要求一个 process 为多个 renderer 提供独立 connection。默认 stdio 只有一条 stream；WebSocket 未正式支持；Unix socket 不能单独证明完整跨平台桌面契约。实现前必须先取得用户对 backend transport 工作的决定。
- **生成 method map**：TypeScript 生成物没有完整 method → params → response map，无法实现不手写映射的 typed protocol client。
- **运行时 decoder**：生成物不能把 `unknown` frame 验证为 envelope/params/result；类型断言不能补这个边界。
- **Project 与 Environment 稳定性**：相关 method 与 Thread 字段当前是 experimental。产品依赖它们前必须由用户决定是否接受版本绑定，或先稳定协议。
- **通用 request cancellation**：后端没有任意 client request 的通用 cancel；已有取消通过 interrupt、terminate、unwatch 和领域 cancel method 表达。
- **结构化领域错误**：部分失败仍只有通用 code/message；前端需要稳定分类时必须先补 data 或专用 code。
- **协议兼容字段**：initialize user agent 可诊断版本，但不是独立 compatibility negotiation；独立升级后端前必须先补稳定兼容契约。
- **server request connection owner**：交互 request 必须回到发起当前工作的 renderer connection；任何跨 connection 投递行为都要先有明确协议 identity 与路由保证。

这些阻塞不能由 Main 共享 protocol connection、每窗口 process、实验 transport 自动切换、手写 decoder、英文消息匹配或前端持久映射掩盖。

## 对 skill 的直接影响

- Main 是 process starter 与透明 connection relay，不是 protocol connection owner。
- renderer protocol client 拥有每条 connection 的 initialize、request pairing、notification、server request 和 close。
- 一个 host process 服务多个 renderer connection；Project、Workspace 和 window 不创建 process。
- 每个前端领域保留自己的 service，并通过所属领域 app-server adapter 使用 renderer host service；Sessions Provider 不是通用 adapter。
- 只有 Thread 进入 Agents Window 时，Sessions Provider 才把 Thread 投影为 Provider-neutral facade。
- 此时 Session ID 从 provider ID 与 provider-owned resource 生成；committed resource 承载 Thread identity，不保存第二份映射。
- 此时 backend Thread 默认投影为 Session 主 Chat；multi-chat capability 需要独立 Chat catalog，而不是把 Thread 强行拼组。
- 此时 Project、Workspace、Environment、Provider、Session 与 Thread identity 必须分开。
- Rust 协议是 method 与 DTO 唯一 owner；renderer protocol client 依赖生成 method map 与 decoder。
- call cancellation 不等于后端取消；领域 interrupt/stop/unwatch/terminate/cancel 才结束工作。
- 后端落盘修改与 dirty model 相遇时由现有 file/working-copy owner 处理，领域 adapter 不建立文件状态副本。

## 重新核对源码的条件

仅在以下情况重新打开对应源码：

- 前端的 per-renderer MessagePort、protocol client、领域 service 边界改变，或 Thread UI 涉及的 Provider contract、Session replacement 改变；
- 后端新增正式跨平台多 connection 本地 transport，或改变 stdio/socket 支持级别；
- Project、Environment 或 Thread assignment 从 experimental 变为稳定，或字段语义变化；
- 协议 generator 增加 method map、runtime decoder 或 compatibility negotiation；
- backend 增加通用 request cancellation，或 server request 改变 connection routing；
- reference 无法回答一个会改变 owner、文件位置或用户可观察行为的问题。

重新核对发现源码、owner、协议或用户改动冲突时，执行主 skill 的冲突门禁。核对后只把稳定结论更新到对应 reference，不复制参考实现或维护逐行源码摘要。
