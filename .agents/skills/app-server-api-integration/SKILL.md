---
name: app-server-api-integration
description: Design, implement, or review a TypeScript desktop frontend connected to a Rust app-server. Use for process and session ownership, typed bidirectional messages, generated protocol types, domain channels, cancellation, subscriptions, host requests, lifecycle, and exact file placement; do not use for UI design, frontend domain behavior, or Rust domain internals.
---

# App-server API 对接

按最终架构设计桌面前端与 app-server 的边界，不沿用现有目录、调用链或错误所有权。先确定领域契约、进程边界、线上协议和资源生命周期，再落文件；现有实现冲突时让错误 owner 退场，不增加转发层、双写或备用后端。

## 架构基线

```text
renderer 领域服务
  → 命名 channel client
  → renderer ↔ Main IPC
  → Main 领域 channel
  → session 对应的共享 connection
  → stdio JSONL transport
  → 双向线上协议
  → 统一消息分派
  → 领域 processor
  → Rust 领域能力
```

这条链包含两份契约：renderer 使用前端领域契约，Main 与 app-server 使用生成的线上契约。Main 领域 channel 是唯一同时理解两者的适配点；不能把线上 method、DTO、错误 envelope 或 request ID 暴露给 renderer。

## 不变量

- renderer 不持有后端进程、transport、connection、request ID 或 pending map。
- renderer ↔ Main IPC 与 Main ↔ app-server 协议相互独立，不能共用 method 常量或通用调用入口。
- Electron Main 独占进程、session、connection 选择、初始化、请求配对、通知分派和服务端主动请求路由。
- 一个后端 session 只有一个 connection owner；领域不能分别启动进程或解析 stdout。
- Rust 协议注册与生成器是线上 method、params、response、notification 和 server request 的唯一来源。
- 前端领域接口由其领域目录拥有；使用 app-server 不会把领域职责转移到 `platform/appServer`。
- adapter 只做机械转换与生命周期衔接，不做业务决定、持久化、权限判断或静默重试。
- 通用 connection request 不接收 `CancellationToken`；取消必须映射为协议已有的领域 interrupt、stop、unwatch、terminate 或 cancel request。
- 断线使当前 connection 的 pending、资源和路由全部失效。新 connection 使用新代次，旧请求和修改操作不自动重放。

## 工作流程

1. 写出 renderer 真正需要的领域接口、事件、错误、取消和释放行为，不从当前实现或线上 DTO 反推接口。
2. 确定 session 隔离键、renderer 归属和 Main 中唯一的 connection owner。
3. 为每项行为选择 request、notification、显式资源或 server request，并定义稳定 ID、顺序、错误和关闭结果。
4. 检查 Rust 注册点能否生成方法到参数和返回值的完整映射；生成物不完整时先改生成器并停止前端手写映射。
5. 在 Main 领域 channel 中转换两份契约，在 renderer 注册领域 channel client；产品入口只创建服务和注册 channel。
6. 验证初始化、请求配对、事件缺口、领域取消、renderer 断开、多窗口路由、connection 关闭、背压和生成物同步。

## Reference 路由

| 当前任务 | 必须读取 |
| --- | --- |
| 连接、进程、session、多窗口、所有权或文件位置 | [连接与所有权](references/connection-architecture.md) |
| request、notification、资源、取消、错误、顺序或 server request | [协议语义](references/protocol-semantics.md) |
| 新增或修改 API、实现代码或 review | [实现流程](references/implementation-template.md) |
| 修改本 skill、核对依据，或 reference 无法回答关键边界 | [源码证据](references/source-evidence.md) |

完整实现一项领域 API 时读取前三份。普通对接任务不重复研究全部参考源码；只有 [源码证据](references/source-evidence.md) 列出的重新核对条件成立时才回到源码。

## 立即拒绝的设计

- renderer 直接读写 JSONL、调用线上 method、导入生成 DTO 或处理 server request union。
- `invoke(method, unknown)`、通用业务 API、手写响应泛型或手写 method → response 表。
- 把后端 JSON-RPC 直接注册成一个万能 `IChannel`，或让一个领域 channel 暴露全部后端方法。
- 每个领域维护自己的 transport、初始化状态、request ID、pending map、reader 或重连循环。
- 忽略 renderer context，广播审批/输入请求，或让任意 workbench contribution 注册线上 handler。
- 把 IPC 的 Promise cancel 当作后端工作已经取消，或把丢弃 Promise、忽略迟到 response 当作取消。
- 用英文错误消息分类、把 notification 当 response、把无限事件表示成永不结束的 Promise。
- 关闭后保留旧资源、静默重放修改请求、自动切换实现路径或维持新旧 owner 并存。

## 完成标准

最终答复必须给出：前端领域契约、线上消息形态、session 和 connection owner、renderer 归属、准确文件位置、生成物更新、取消与关闭语义、实际运行的测试，以及仍阻止正确实现的协议缺口。发现当前结构冲突时，列出需要退场的 owner 和调用入口，不为它们设计兼容层。
