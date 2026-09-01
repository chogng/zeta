# 源码证据

本文件只用于维护本 skill、核对依据或处理其他 reference 未覆盖的边界。普通 API 对接直接读取功能 reference，不重复研究全部参考源码。

路径采用两个中性根：前端参考源码根以下写作 `src/`，后端参考 crate 使用 `../app-server*`。这些路径证明职责和行为，不要求复制实现，也不表示目标仓库已经具备对应文件。

## 前端进程架构

| 路径 | 已验证结论 |
| --- | --- |
| `src/base/parts/ipc/common/ipc.ts` | `IChannel.call/listen` 与 `IServerChannel` 构成命名领域 channel；IPC client 统一拥有 request ID、pending handler、Promise cancel、event subscribe/unsubscribe 和 dispose。 |
| `src/base/parts/ipc/electron-main/ipc.electron.ts` | Main 为每个 renderer connection 创建消息协议，并以 renderer 断开事件结束该 IPC connection。 |
| `src/base/parts/ipc/electron-browser/ipc.electron.ts` | renderer 只创建 IPC client，通过命名 channel 取得服务；dispose 显式断开。 |
| `src/platform/ipc/electron-browser/services.ts` | renderer 把命名 channel 注册为领域 service；涉及转换和生命周期时使用显式 channel client，简单服务才使用代理。 |
| `src/platform/update/common/updateIpc.ts` | 领域 channel client 可以恢复状态并把 IPC event 转为领域 event；server channel 把 command/event 映射到真实 service。 |
| `src/platform/request/common/requestIpc.ts` | token 能到达 server channel，但跨边界 buffer/stream 需要显式转换。 |
| `src/platform/userDataSync/common/userDataSyncIpc.ts` | URI、初始状态和领域对象在 channel client 恢复，调用方不处理 IPC envelope。 |
| `src/platform/meteredConnection/common/meteredConnectionIpc.ts` 与 `src/platform/meteredConnection/electron-main/meteredConnectionChannel.ts` | `common` 可以拥有 IPC 名称和 renderer client，Main 文件拥有进程侧 channel。 |
| `src/code/electron-main/app.ts` | 产品入口集中创建 service 和注册 channel，但不实现 IPC primitive、协议 parser 或领域 service。 |

`ProxyChannel` 不支持 `CancellationToken`，也只恢复少量通用类型。涉及取消、错误、资源、状态恢复、renderer context 或线上 DTO 转换时必须使用显式领域 channel，不能用反射式代理隐藏边界。

`IPCServer` 在每个 renderer connection 上同时拥有 channel server 和 channel client，因此 Main 可以按 connection context 反向调用 renderer。这个能力支持 server request 路由，但不提供 owner 选择；thread/turn/resource 到 renderer 的归属仍由 Main session owner 建立。

## 后端协议和运行时

| 路径 | 已验证结论 |
| --- | --- |
| `../app-server/README.md` | 线上协议双向通信并省略标准 JSON-RPC 版本字段；stdio 默认使用逐行 JSON，WebSocket 仍是实验 transport；每条 connection 必须完成 `initialize` → `initialized`。 |
| `../app-server-protocol/src/protocol/common.rs` | 一个注册点绑定 typed client request、response、server notification、server request 和 serialization scope；客户端不应重复维护 method 与 DTO。 |
| `../app-server-protocol/src/export.rs` 与 `../app-server-protocol/src/precomputed_exports.rs` | TypeScript 与 JSON Schema 由协议 crate 生成，生成物不可手改。 |
| `../app-server-protocol/src/protocol/v2/fs.rs` | watch 使用 client 提供的 connection-scoped `watch_id`，start/unwatch request 与 notification 共用该 ID。 |
| `../app-server-protocol/src/rpc.rs` | 线上错误包含 numeric code、message 和可选结构化 data；需要稳定领域分类时不能只依赖 message。 |
| `../app-server/src/message_processor.rs` | initialize 是每 connection 门禁；其他 request 在初始化前被拒绝；统一 processor 做 typed dispatch。 |
| `../app-server/src/request_serialization.rs` | 串行 key 由协议 scope 产生；connection-scoped process/watch key 包含 connection ID；不同 key 并行，共享读不能越过排队写。 |
| `../app-server/src/connection_rpc_gate.rs` | connection 关闭后不再启动排队 request，并等待已经开始的 request 收尾。 |
| `../app-server/src/outgoing_message.rs` | outgoing owner 统一发送 response/error/notification，维护 server request callback，并按 connection 路由。 |
| `../app-server/src/fs_watch.rs` | 资源以 `(connection_id, watch_id)` 隔离；重复 ID 被拒绝；unwatch 等待任务结束后响应；connection close 清理资源。 |
| `../app-server/src/transport.rs` | transport connection 状态和 outbound 路由独立于领域 processor；普通通知只发送到已初始化 connection。 |
| `../app-server-transport/src/transport/mod.rs` | transport 使用有界队列；输入过载返回稳定错误，慢 connection 可以被断开；transport 只产生 opened/closed/message。 |
| `../app-server-transport/src/transport/stdio.rs` | stdio 只负责逐行 framing、读取、写出和关闭，不处理 initialize 或领域 method。 |
| `../app-server-client/src/remote.rs` | client connection owner 统一完成 initialize、request pairing、notification 流、server request resolution 和断线后 pending rejection；握手期间暂存已知 notification/server request，未知 server request 立即拒绝，响应后发送 `initialized`。 |

## 当前协议缺口

- 生成的 TypeScript 有 request union 和独立 response 类型，但没有完整的 method → params → response map。通用 typed connection 实现前必须扩展生成器。
- 后端没有接收任意 client request 的通用 `$/cancelRequest`。已有取消通过 `turn/interrupt`、terminate、unwatch 和领域 cancel method 表达。
- 错误 envelope 支持结构化 data，但部分领域失败仍只有通用 code/message。前端需要稳定分类时必须先补 data 或专用 code。
- 正式桌面传输应使用 stdio；实验 transport 不能作为默认、自动回退或浏览器直连方案。
- 初始化响应中的 user agent 可提供诊断版本，但协议没有独立兼容性协商字段；生成物必须与打包的后端来自同一版本。

这些缺口是实现阻塞或协议工作项，不能由前端手写表、匹配英文消息、伪造取消或增加备用链路掩盖。

## 对 skill 的直接影响

- renderer IPC 与 app-server transport 是两段协议，只在 Main 领域 channel 相遇。
- connection、双向 pending、initialize 和消息分类是共享机制，不能按领域复制。
- 前端 IPC 已经拥有 call/listen/dispose；对接层补充的是后端 connection、session 和领域适配器，不重写 IPC。
- Main 必须使用 renderer context 管理 attachment 和反向请求；active window 不能成为路由依据。
- Rust 协议是线上方法和 DTO 的唯一 owner，TypeScript connection 依赖生成 map。
- call cancellation 不等于后端取消；领域 interrupt/stop/unwatch/terminate/cancel 才结束后端工作。
- 长期资源由 connection ID 与 resource ID 共同隔离，并在 stop、renderer disconnect 和 connection close 时确定清理。
- 文件位置由职责决定：通用 IPC、connection、stdio、session、领域 channel、协议、统一分派和 processor 分别拥有自己的边界。

## 重新核对源码的条件

仅在以下情况重新打开对应源码：

- 前端 IPC 的 call/listen/dispose、connection context 或反向 channel 契约变化；
- 协议 registry、生成器、error envelope 或初始化握手变化；
- 新 transport 改变 framing、背压、正式支持级别或连接数量；
- 后端增加通用 client request cancellation；
- server request 增加取消、超时或 connection 定向语义；
- reference 无法回答一个会改变 owner、文件位置或用户可观察行为的问题。

核对后只把稳定结论更新到对应 reference，不复制参考实现或维护逐行源码摘要。
