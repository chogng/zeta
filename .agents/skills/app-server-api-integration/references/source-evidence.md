# 源码证据

本文件只用于维护本 skill、核对设计依据或解决 reference 未覆盖的行为。普通 API 对接直接读取其他 reference，不重复研究所有源码。

以下路径是研究时使用的源码相对路径，不是 Zeta 当前实现状态，也不是要求复制文件名。

## 前端进程架构

| 路径 | 已验证结论 |
| --- | --- |
| `src/vs/base/parts/ipc/common/ipc.ts` | `IChannel.call`、`listen` 与 `IServerChannel` 构成领域 channel；client request ID、pending handler、call cancel、event subscribe/unsubscribe 和连接 dispose 由 IPC 层统一拥有。 |
| `src/vs/base/parts/ipc/electron-main/ipc.electron.ts` | Electron Main 按 renderer connection 创建 channel server/client，并在 renderer disconnect 时统一释放。 |
| `src/vs/base/parts/ipc/electron-browser/ipc.electron.ts` | renderer 只创建进程 IPC client，通过 channel 访问服务；dispose 会显式断开协议。 |
| `src/vs/platform/ipc/electron-browser/services.ts` | renderer 把命名 channel 注册成领域 service；复杂转换使用明确的 channel client，简单 service 才使用通用代理。 |
| `src/vs/platform/update/common/updateIpc.ts` | 领域 channel client 可以恢复状态并把 IPC event 转成领域 event；server channel 把 command/event 映射到实际 service。 |
| `src/vs/platform/request/common/requestIpc.ts` | 取消 token 到达 server channel，但跨边界的数据流需要显式 buffer/stream 转换。 |
| `src/vs/platform/userDataSync/common/userDataSyncIpc.ts` | URI、初始状态和领域对象在 channel client 恢复，调用方不处理 IPC envelope。 |
| `src/vs/platform/meteredConnection/common/meteredConnectionIpc.ts` 与 `src/vs/platform/meteredConnection/electron-main/meteredConnectionChannel.ts` | `common` 拥有 IPC 名称和 renderer client，Electron Main 文件拥有进程侧 channel。 |
| `src/vs/code/electron-main/app.ts` | 产品入口集中创建 service 和注册 channel，但不实现 IPC primitive 或领域 service。 |

`ProxyChannel` 明确不支持 `CancellationToken`，并且只自动恢复少量通用类型。因此涉及取消、错误、资源、状态恢复或协议 DTO 转换时必须使用显式领域 channel，不能用反射式代理隐藏边界。

## 后端协议与运行时

| 路径 | 已验证结论 |
| --- | --- |
| `../app-server-protocol/src/protocol/common.rs` | 一个注册点生成 typed client request、response、server notification、server request 和 serialization scope；方法名与 DTO 不应在客户端重复维护。 |
| `../app-server-protocol/src/precomputed_exports.rs` | TypeScript 与 JSON schema 由协议 crate 生成，生成文件带不可手改标记。 |
| `../app-server-protocol/src/protocol/v2/fs.rs` | 持续 watch 使用 client 提供的 `watch_id`，start/unwatch request 与 notification 共用该 ID。 |
| `../app-server/src/message_processor.rs` | initialize 是每 connection 门禁；未初始化 request 被拒绝；统一 processor 做 typed dispatch，并在 connection 关闭时清理领域资源。 |
| `../app-server/src/request_serialization.rs` | 串行 key 由协议 scope 产生；connection-scoped process/watch key 包含 connection ID；不同 key 并行，共享读不能越过排队写。 |
| `../app-server/src/connection_rpc_gate.rs` | connection 关闭后不再启动排队 request，并等待已经开始的 request 收尾。 |
| `../app-server/src/outgoing_message.rs` | outgoing owner 统一发送 response/error/notification，维护 server request callback，并按 connection 路由。 |
| `../app-server/src/fs_watch.rs` | 资源以 `(connection_id, watch_id)` 隔离；重复 ID 被拒绝；unwatch 等待任务结束后才响应；connection 关闭清理资源。 |
| `../app-server/src/transport.rs` | transport 连接状态和 outbound 路由独立于领域 processor；只向已初始化 connection 广播普通通知。 |
| `../app-server-transport/src/transport/mod.rs` | transport 使用有界队列；输入过载返回稳定错误，慢 connection 可以被断开；transport 只产生 opened/closed/message 事件。 |
| `../app-server-transport/src/transport/stdio.rs` | stdio 只负责逐行 framing、读取、写出和关闭事件，不处理领域 method。 |
| `../app-server-client/src/remote.rs` | client connection owner 统一完成 initialize、pending request pairing、notification 流、server request resolution 和断线后 pending rejection。 |

## 对 skill 的直接影响

源码证据固定了以下设计：

- renderer IPC 与 app-server transport 是两段独立协议，只在 Electron Main 领域 channel 相遇。
- connection、pending map、初始化和消息分类是共享机制，不能按领域复制。
- Rust 协议是线上方法和 DTO 的单一 owner，TypeScript 需要生成的 method→response 映射。
- 前端 call cancellation 不等于后端取消；后端当前使用领域 interrupt、stop、unwatch、terminate 或 cancel request。
- 长期资源必须由 connection ID 与资源 ID 共同隔离，并在 stop 和 connection close 时确定清理。
- 文件位置由职责决定：通用 IPC 在 `base/platform ipc`，领域接口和 channel 在领域目录，产品入口只装配，协议、transport、统一分派和领域 processor 各自独立。

## 重新核对源码的条件

仅在以下情况重新打开对应源码：

- 协议 registry、生成器或初始化握手已经变化；
- 新 transport 引入不同 framing、背压或连接数量；
- 后端增加通用 request cancellation；
- server request 开始具备取消、超时或 connection 定向的新语义；
- 前端 IPC 的 call/listen/dispose 契约变化；
- 当前 reference 无法回答一个会改变所有权或文件位置的问题。

核对后把稳定结论更新到对应 reference；不要把源码逐行抄入 skill。
