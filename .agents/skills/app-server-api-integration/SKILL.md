---
name: app-server-api-integration
description: Design, implement, or review Zeta's TypeScript-to-Rust app-server boundary. Use for process ownership, typed requests, notifications, subscriptions, cancellation, server-initiated requests, generated protocol types, adapters, lifecycle, and exact file placement; do not use for UI design, frontend domain behavior, or Rust domain internals.
---

# App-server API 对接

本 skill 设计 Zeta 的最终前后端边界，不以当前目录、现有调用链或兼容旧实现为起点。先确定领域契约、进程所有权和线上协议，再决定代码落位；当前实现与最终结构冲突时，指出冲突并按最终结构设计，不增加兜底路径、双写或旧后端回退。

## 最终边界

```text
renderer 领域服务
  → 命名 channel 的 client
  → 进程 IPC
  → Electron Main 领域 channel
  → 共享 app-server connection
  → 有类型的线上协议
  → 统一消息分派
  → Rust 领域 processor
  → Rust 领域能力
```

- renderer 只依赖前端领域接口，不导入线上 DTO、不持有连接、不生成请求编号。
- Electron Main 独占后端进程、transport、初始化、请求配对、通知分派和服务端主动请求。
- Rust 协议注册是方法名、参数、响应、通知、服务端主动请求和串行范围的唯一来源；TypeScript 只消费生成物。
- 领域 Electron Main adapter（channel 或 server request handler）是两段协议的唯一转换点，只转换类型、错误、事件、资源标识和生命周期，不决定业务。
- 每个后端 session 使用一条共享 connection；领域不能各自启动进程、读取 transport 或维护 pending map。

完整所有权、连接状态和目标目录见 [连接与所有权](references/connection-architecture.md)。

## 工作方式

1. 写出最终领域契约：方法、事件、状态、错误、取消和释放行为。不要默认沿用当前接口。
2. 选择 request、notification、显式资源订阅或 server request，并确定稳定资源 ID 和并发范围。
3. 在 Rust 协议注册处定义线上契约并生成 TypeScript 类型；生成器缺少方法到响应的映射时，修改生成器，不手写第二份映射。
4. 在 Rust 统一分派中接入领域 processor；业务规则留在领域能力中。
5. 在 Electron Main 的领域 channel 中连接共享 connection，并把线上 DTO 转为前端领域类型。
6. 在 renderer 注册领域 channel client，使调用方继续只看到领域服务。
7. 验证初始化、请求配对、事件路由、取消、释放、服务端主动请求、连接关闭和生成物同步。

## 按任务读取 reference

| 任务 | 必须读取 |
| --- | --- |
| 设计连接、进程、生命周期、所有权或文件位置 | [连接与所有权](references/connection-architecture.md) |
| 设计 request、notification、订阅、取消、错误、顺序或 server request | [协议语义](references/protocol-semantics.md) |
| 新增或修改一条 API、写代码或做 review | [实现流程](references/implementation-template.md) |
| 修改本 skill、核对设计依据或遇到 reference 未覆盖的行为 | [源码证据](references/source-evidence.md) |

完整新增一个领域 API 时读取前三份。只在维护 skill 或设计依据出现歧义时读取源码证据并回到对应源码；不要在普通对接任务中重复研究全部源码。

## 不接受的设计

- renderer 直接调用 JSON-RPC、导入生成 DTO 或处理后端方法名。
- 通用 `IAppServerApi`、`invoke(method, unknown)` 或集中暴露全部业务的 service。
- 在 TypeScript 手写方法字符串、请求参数、响应类型、通知表或 server request 表。
- 每个领域各自持有 transport、初始化状态、请求编号、pending map 或重连循环。
- 把通知当响应、把无限事件当永不结束的 Promise、把丢弃 Promise 当取消。
- 用英文错误消息判断错误类别，或让 JSON-RPC envelope 进入 editor、workbench 和 UI。
- 断线后静默重放修改请求、假装旧资源仍存活，或切回另一条实现路径。

## 完成标准

最终答复必须说明：领域契约、线上消息形态、连接与资源 owner、准确文件位置、取消和关闭语义、生成物更新方式、实际测试，以及仍阻止正确实现的协议缺口。若当前结构与目标结构冲突，直接列出需要退场的所有权，不为旧结构设计兼容层。
