# 协议语义

本文件规定如何把前端领域行为映射成 Rust app-server 消息。先定义调用方能观察到的行为，再选择线上消息；不要从当前 method 或 DTO 反推领域接口。

## 行为表

每条对接先写清楚：

| 项目 | 必须回答 |
| --- | --- |
| 领域行为 | 调用方要完成什么，返回值和可观察副作用是什么 |
| owner | 哪个 Rust 领域能力校验输入、决定行为并保存状态 |
| 消息形态 | request、notification、资源订阅、stream 或 server request |
| 标识 | request ID 之外是否需要 operation、resource、revision 或 command ID |
| 生命周期 | 谁创建、谁释放，window、session 或 connection 关闭时如何结束 |
| 取消 | 取消哪项后端工作，取消与正常完成竞争时哪个结果生效 |
| 顺序 | 哪些动作必须按稳定 key 串行，哪些必须并行 |
| 错误 | 稳定错误 code/data 如何转换成前端领域错误 |
| 恢复 | 断线后失败、重读、重订阅还是由协议提供恢复 |

## 消息选择

| 前端语义 | 线上消息 | 必须具备 |
| --- | --- | --- |
| 一次性查询或修改 | typed request/response | params、response、稳定错误、必要的串行范围 |
| connection 期间的全局变化 | typed server notification | 明确触发条件；初始化后的可用范围 |
| watch、process、terminal、search 等持续资源 | start request + 带 resource ID 的 notification + stop request | 稳定 ID、终止信号、connection cleanup |
| Rust 需要前端返回值 | typed server request/response | request ID、typed response、错误和关闭语义 |
| 取消既有工作 | 领域明确的 interrupt、stop、unwatch、terminate 或 cancel request | 指向稳定 operation/resource ID |

notification 不用于需要确认成功的命令；request response 不用于持续流；server request 不用“通知 + 另一个 client request”模拟。

## 一次性 request

协议注册点一次性绑定 method、params、response 和 serialization scope。TypeScript connection 的请求接口必须由生成映射约束：

```ts
interface IAppServerConnection {
	request<M extends keyof AppServerRequestMap>(
		method: M,
		params: AppServerRequestMap[M]['params'],
	): Promise<AppServerRequestMap[M]['response']>;
}
```

`AppServerRequestMap` 是生成物。connection 实现可以组装 JSON-RPC envelope，但调用方不能传 `unknown` params 或自行指定 response 泛型。

request ID 只负责当前 connection 内的响应配对。需要幂等或未知结果恢复的修改必须另有领域 `commandId`、revision 或稳定对象 ID；不能把 request ID 当业务身份。

## Notification

connection reader 必须先完成线上消息分类，再把 typed notification 分派给同步、轻量的本地 listener。UI listener 不能阻塞 reader；需要异步工作的 adapter 自己排队并拥有队列上限。

全局 notification 只表示“某件事发生了”，不自动构成完整状态。需要可靠当前值时，领域契约应提供 snapshot request，或由订阅 start response 返回 snapshot/revision。不能依赖“先请求快照还是先注册 listener”的时间巧合避免事件缺口。

## 显式资源订阅

持续资源使用由 client 生成的稳定 resource ID，使 start 发送前即可建立路由和取消目标：

```text
创建 resource ID
  → 注册本地 notification 路由
  → start(resource ID, params)
  → 接收 notification(resource ID, ...)
  → completed/exited 或 stop(resource ID)
  → 删除路由
```

- 后端资源 key 至少包含 connection ID 和 resource ID，避免不同 client 相互停止资源。
- 重复 resource ID 在同一 connection 内必须报错，不能覆盖旧资源。
- stop response 返回前要保证之后不会再发该 resource 的 notification。
- connection 关闭必须释放该 connection 创建的全部资源。
- renderer 的最后一个 listener dispose 时，Electron Main adapter 才发送 stop；多个本地 listener 共享同一个后端资源。
- start 失败、stop 失败和 connection 关闭都必须终止前端 `Event` 并清理本地路由。

需要补发或检测缺口时，notification 带单调 sequence，snapshot 带 revision，并明确从哪个 sequence 继续。没有恢复需求的同 connection 实时事件不机械增加持久序号。

## 取消

renderer IPC 的 `CancellationToken` 只会取消 renderer ↔ Main 的 call。当前 app-server 消息没有通用 `$/cancelRequest` 语义，因此 connection 的通用 `request` API 不接收 `CancellationToken`，避免让调用方误以为任意 Rust request 可取消。

需要取消的领域操作必须满足：

1. client 在开始前生成 operation/resource ID；
2. start request 携带该 ID；
3. Electron Main 监听 `CancellationToken`，发送对应的 typed cancel request；
4. Rust processor 把 cancel 交给实际工作 owner；
5. 协议定义取消与正常完成竞争时的唯一终态；
6. adapter 在完成、取消或关闭后释放 token listener。

若后端只支持“停止等待”而工作继续运行，前端领域契约必须明确表达这一点；不能把它命名为取消。丢弃 Promise、忽略迟到 response 或发送未知 method 都不构成取消。

## Server request

Rust 需要宿主批准、输入或执行宿主能力时发送 typed server request。connection owner 按生成 method 注册 handler，并保证每个 request 恰好回复一次成功或错误。

| handler 所需能力 | 放置位置 |
| --- | --- |
| Electron Main 已拥有的系统或进程能力 | 对应 `src/platform/<domain>/electron-main/` handler |
| renderer 才能完成的交互 | 独立 host channel 转发到明确的 workbench service |
| Rust 自己可以完成的领域行为 | 不发送 server request，直接调用 Rust 领域能力 |

未知 method 立即返回 method-not-found。handler 超时、window 关闭和 connection 关闭返回稳定错误；不能让 server request 永久占用 pending map。普通 workbench contribution 不能注册任意线上 handler。

## 顺序与并发

顺序由 Rust 协议的 serialization scope 决定，常见 key 包括 thread、path、process、watch 和 session。需要跨 connection 共享的资源使用全局 key；只属于 connection 的 process/watch key 同时包含 connection ID。

- 同 key 的修改按协议定义串行。
- 明确标为共享读的 request 可以并行，但不能越过已经排队的写。
- 不同 key 默认并行；共用一条 connection 不代表全局串行。
- 前端不得再添加另一套领域锁来修补后端顺序。

如果顺序影响业务正确性，serialization scope、revision 或领域锁必须位于 Rust owner；前端调用先后不构成协议保证。

## 错误分层

| 层 | 例子 | 处理位置 |
| --- | --- | --- |
| transport | EOF、进程退出、写失败、队列满 | connection owner 关闭并拒绝 pending |
| protocol | method 不存在、params 无效、未 initialize | connection 解码为有类型的 protocol error |
| domain | 文件不存在、状态冲突、权限拒绝 | Electron Main 领域 channel 转为前端领域错误 |
| decode | response 与生成类型不匹配 | 视为前后端版本或实现错误，不伪装成领域失败 |

错误分类依赖稳定 code 和结构化 data，不匹配英文 message。JSON-RPC envelope 和 Rust 内部错误不能进入 renderer。

## 初始化和关闭

connection 建立后先发送 typed `initialize`，校验版本、client identity 和 capabilities，等待成功后再开放领域 request。协议要求 `initialized` notification 时，在 initialize response 后发送；initialize 期间收到的 notification 和 server request 先入队，ready 后按原顺序交付。

关闭时：

1. connection 停止接受新领域 request；
2. reader/writer 停止并记录单一关闭原因；
3. 所有 pending client request 以 transport error 结束；
4. 所有 pending server request 以关闭错误结束；
5. 所有资源路由和 notification listener 失效；
6. Electron Main 领域 adapter 发出领域允许的终止事件或状态变化。

新 connection 重新 initialize。旧 operation/resource ID、pending response 和 notification 不进入新代次；没有协议恢复能力的资源由领域明确重建。

## 背压

transport 的输入和输出队列有明确上限。队列满时，request 获得 overload error，或慢 connection 被关闭；不能无限增长内存。大 payload 在协议中定义分块上限，持续输出使用 resource notification，写出必须尊重 transport drain。
