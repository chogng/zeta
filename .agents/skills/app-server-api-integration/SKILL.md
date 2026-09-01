---
name: app-server-api-integration
description: Design, implement, or review the boundary between a TypeScript desktop frontend and a Rust app-server. Use for frontend domain services, adapters, renderer/Main ownership, process and connection topology, generated bidirectional protocol, cancellation, host requests, lifecycle, conflict gating, and exact file placement. Use Sessions rules only when backend threads are surfaced in the Agents Window. Do not use for UI design, frontend feature behavior, or Rust domain internals.
---

# App-server API 对接

保留 TypeScript 前端的 `base → platform → editor → workbench` 分层、renderer/Main 进程模型、领域服务和产品装配，用 Rust app-server 替换 TypeScript 后端运行层。每个前端领域继续拥有自己的公共契约；app-server 只替换后端能力与线上协议，不成为所有前端领域的公共 API。

## 架构基线

```text
renderer contribution 或调用方
  → 前端领域 service
  → 该领域的 app-server adapter
  → renderer app-server host service
  → renderer protocol client
  → 每个 renderer 独立 transport/connection
  → Main process starter 与透明 relay
  → 一个共享 app-server 进程
  → typed 双向协议与 Rust processor
```

前端领域契约与线上协议服务不同调用者。protocol client 处理生成的 envelope、初始化、请求配对、notification 和 server request；领域 adapter 只把生成的后端类型转换成前端领域类型。文件、账户、配置、模型、Skills、Project、Thread、process 等能力分别接入自己的领域 service，不能统一经过 Sessions Provider 或万能 app-server service。

只有把 backend Thread 显示到 Agents Window 时，才增加 Sessions Provider，把 Thread 投影成 Provider-neutral `ISession`/`IChat`。Sessions 是条件消费者，不是 app-server 对接主干。

## 不变量

- 前端领域接口、状态和可见错误继续由对应 `src/platform/<domain>/`、`src/workbench/services/<domain>/` 或 `src/sessions/` owner 持有；`src/platform/appServer/` 只拥有共享 host/connection 机制。
- Main 独占进程启动、停止、可执行文件解析、renderer connection acquisition 和透明 relay；Main 不拥有线上 request ID、pending map、initialize、notification 分类或领域状态。
- 一个 host 默认启动一个进程；每个 renderer 获取一条独立 backend connection，并在 renderer 内创建唯一 protocol client。窗口、Project、Workspace 或领域不能成为新进程 key。
- 正式实现必须有受支持的多 connection 本地 transport。只有单路 stdio、实验 transport 或平台不完整的 endpoint 时停止依赖它的前端实现；已授权后端前置工作时按 [前置能力补全](references/prerequisite-completion.md) 完成正式路径，否则执行冲突门禁。不能改成 Main 共享线上 connection、每窗口一进程或万能 IPC 协议代理。
- Rust 协议注册与生成器是 method、params、response、notification 和 server request 的唯一来源，并同时生成编译期 method map 与运行时 decoder。
- 每个领域 adapter 只做机械转换与生命周期衔接，不做业务决定、持久化、权限判断、静默重试或协议兼容猜测。
- 后端拥有 durable Project、Thread、Turn、Item、process/resource 和执行状态；前端只保存其领域 facade 与 UI 状态，不复制后端持久化 owner。
- Project、Workspace、Environment、Thread 和 connection 身份不得合并。Project 不表示 Session Host；Workspace 不表示 Project catalog；Environment 是 Thread/Turn 的执行目标。
- Sessions 对接存在时，canonical Session ID 从 `providerId + provider-owned resource` 生成；committed resource 直接承载 backend Thread identity，不另存 frontend Session ID → backend Thread ID 映射。
- 一个 backend Thread 默认投影为一个 Session 的主 Chat。只有后端明确提供 Chat catalog、稳定 Chat ID、恢复和生命周期协议时才声明 multi-chat。
- 取消必须映射为协议已有的 interrupt、stop、unwatch、terminate 或领域 cancel request；丢弃 Promise 不代表后端工作已经结束。
- renderer connection 关闭只清理该 connection 的 pending、资源和订阅，不停止共享进程；进程退出使全部 renderer connection 失效。

## 冲突门禁

实现、迁移或 review 前检查两侧源码、目标 owner、现有公开 API、用户已有改动和所需协议语义。[源码证据](references/source-evidence.md) 已记录且 [前置能力补全](references/prerequisite-completion.md) 已定义的缺口，在用户授权对应后端范围时按计划先补后端；除此以外出现以下任一情况时立即停止修改，只继续只读调查并一次性报告完整冲突清单：

- 两个权威源码对同一行为、路径、生命周期或 owner 给出不兼容答案；
- 正式多 connection transport 不存在，且用户未授权补全后端，或目标 backend 不接受计划中的 loopback WebSocket 正式化；
- 所需 API 或字段仍是实验协议，而用户未确认是否依赖或先稳定它；
- 所需行为缺少 method、稳定 ID、运行时 decoder、结构化错误、取消、资源终止或版本兼容语义，且用户未授权修改对应协议 owner，或现有源码无法按前置能力计划唯一补全；
- 领域状态、Project、Thread、resource、renderer 或 connection 无法唯一归属；
- Sessions 对接需要的 Session/Chat/Thread 映射没有唯一身份或完整生命周期；
- 当前公开 API、文件位置或用户改动与目标修改重叠，继续需要猜测保留、迁移、改名或删除选择。

报告准确路径、符号、两种行为、影响范围和需要用户决定的边界，然后等待用户选择。不能自行选边、增加桥接或备用路径、维持双 owner，或先实施依赖该决定的其他修改。

## 工作流程

1. 执行冲突门禁；存在冲突时停在冲突报告。
2. 找到 renderer 调用方真正依赖的前端领域 service，固定其接口、状态、错误、事件、取消和释放语义。
3. 固定一个进程、多 renderer 独立 connection 的拓扑，以及 renderer/process 关闭语义。
4. 为每项领域行为选择 request、notification、显式资源或 server request，并固定身份与顺序。
5. 检查正式 transport、Rust 注册点、method map、运行时 decoder、初始化兼容、稳定 API、结构化错误和 server request owner；缺失时先按 [前置能力补全](references/prerequisite-completion.md) 完成后端契约。
6. 在 renderer protocol client 处理线上协议，在领域 adapter 中转换前后端契约；产品入口只装配 starter、host service、领域 adapter 和真实 contribution。
7. 仅当 Thread 进入 Agents Window 时实现 Sessions Provider、Session replacement 和 Chat capability。
8. 验证初始化、运行时解码、请求配对、领域事件、取消、多窗口、dirty file 冲突、连接关闭、背压和生成物同步。

## Reference 路由

| 当前任务 | 必须读取 |
| --- | --- |
| 领域 owner、进程、多窗口、connection、Thread/Sessions 条件映射或文件位置 | [连接与所有权](references/connection-architecture.md) |
| request、notification、资源、取消、错误、顺序或 server request | [协议语义](references/protocol-semantics.md) |
| 新增或修改 API、实现代码或 review | [实现流程](references/implementation-template.md) |
| transport、生成器、兼容、实验 API、结构化错误或 server request owner 尚未完成 | [前置能力补全](references/prerequisite-completion.md) |
| 修改本 skill、核对依据，或 reference 无法回答关键边界 | [源码证据](references/source-evidence.md) |

完整实现一项 API 时读取前三份；命中任何后端前置缺口时再读取 [前置能力补全](references/prerequisite-completion.md)。只有 [源码证据](references/source-evidence.md) 列出的重新核对条件成立时才回到参考源码。

## 立即拒绝的设计

- 让所有 app-server API 经过 Sessions Provider、Thread service 或一个万能领域 service。
- Main 持有一条共享线上 connection 并替多个 renderer 配对 request。
- 每个 renderer、Project、Workspace 或领域分别启动 app-server 进程。
- renderer UI 或普通 contribution 直接调用线上 method、处理 raw JSON 或导入 envelope union。
- `invoke(method, unknown)`、通用业务 API、手写 method → response 表或用类型断言代替运行时 decoder。
- 把后端协议注册成万能 IPC channel，或让 Main relay 解析、重写、复用线上 request ID。
- Project 等同于 Session Host、Workspace 或 Environment；随机 frontend Session ID 与 backend Thread ID 组成独立持久映射。
- 广播 approval 或用户输入 request、依赖 active window，或把无 owner 的 server request 交给任意窗口。
- 把 IPC Promise cancel 当作后端取消，或关闭后保留旧资源、静默重放修改 request。
- 用旧后端、实验 transport、单路 stdio 或每窗口进程作为备用路径。

## 完成标准

最终答复必须给出：前端领域契约与 owner、领域 adapter、process/connection owner、renderer 归属、线上消息形态、准确文件位置、生成物与 decoder 更新、取消和关闭语义、dirty file 处理、实际运行的测试，以及仍阻止正确实现的 transport 或协议缺口。涉及 Agents Window 时再给出 Provider、Session/Chat/Thread、Project/Workspace/Environment 映射；未涉及时不得为了架构整齐引入 Sessions。任何冲突必须已取得用户决定；没有决定时不能把任务描述为完成。
