# 协议语义

本文件规定如何把前端领域行为映射成 app-server 消息。先写调用方能观察到的行为，再选择线上消息；不能从已有 method、DTO 或 handler 反推领域接口。

## 行为表

每项对接先回答：

| 项目 | 必须明确 |
| --- | --- |
| 领域行为 | 调用方要完成什么，返回值和可观察副作用是什么 |
| 决策 owner | 哪个 Rust 领域能力校验输入、决定行为并保存状态 |
| 消息形态 | request、notification、显式资源或 server request |
| 标识 | request ID 之外是否需要 operation、resource、thread、turn、revision 或 command ID |
| renderer 归属 | 哪个 attachment 发起，反向请求和事件如何回到正确窗口 |
| 生命周期 | 谁创建、谁释放，renderer、session 或 connection 关闭时如何结束 |
| 取消 | 停止哪项后端工作，取消和正常完成竞争时哪个终态生效 |
| 顺序 | 哪些动作按稳定 key 串行，哪些必须并行 |
| 错误 | 稳定 code/data 如何转换成前端领域错误 |
| 恢复 | 断线后失败、重读、重订阅，还是由协议明确恢复 |

## 线上 envelope 与分类

stdio 每行承载一条 JSON 消息，线上省略标准 JSON-RPC 版本字段。connection 根据方向和字段分类：

| 方向和字段 | 类型 | connection 行为 |
| --- | --- | --- |
| client → server，`id + method + params` | client request | 写入后登记 client pending |
| server → client，`id + result` 或 `id + error` | client response | 只完成匹配的 client pending |
| server → client，`method + params` 且无 `id` | server notification | 分派给 typed listener |
| server → client，`id + method + params` | server request | 调用 typed handler 并回复一次 |
| client → server，`method + params` 且无 `id` | client notification | 只用于协议明确允许的通知，例如 `initialized` |

client request ID 与 server request ID 分属两个方向，可以出现相同值；实现必须使用两张 pending 表，不能用一张 map 混合。未知 response ID、重复 response、无效 envelope 或生成类型解码失败属于协议错误，不转成领域失败。

## 消息选择

| 前端语义 | 线上消息 | 必须具备 |
| --- | --- | --- |
| 一次性查询或修改 | typed request/response | params、response、稳定错误、必要串行范围 |
| connection 期间的全局变化 | typed server notification | 明确触发条件和初始化后的可用范围 |
| watch、process、terminal、search 等持续资源 | start request + resource notification + stop request | client-generated ID、终止信号、connection cleanup |
| Rust 需要宿主返回值 | typed server request/response | request ID、唯一 renderer owner、typed response、错误和关闭语义 |
| 取消既有工作 | typed interrupt、stop、unwatch、terminate 或 cancel request | 指向稳定 operation/resource ID |

notification 不用于需要确认成功的命令；request response 不承载无限流；server request 不用“通知 + 另一个 client request”模拟。

## 一次性 request

协议注册点一次绑定 method、params、response 和 serialization scope。TypeScript connection 只能通过生成映射调用：

```ts
interface IAppServerConnection {
	request<M extends keyof AppServerRequestMap>(
		method: M,
		params: AppServerRequestMap[M]['params'],
	): Promise<AppServerRequestMap[M]['response']>;
}
```

`AppServerRequestMap` 必须是生成物。调用方不能传 `unknown` params、自行指定 response 泛型或手写 method 字符串。

request ID 只负责当前 connection 的响应配对。需要幂等、去重或未知结果恢复的修改必须另有 `commandId`、revision 或稳定对象 ID；不能把 request ID 当作业务身份。

## Notification 与快照

connection reader 先完成消息分类和类型校验，再把 notification 分派给同步、轻量的本地 listener。UI listener 不能阻塞 reader；需要异步处理的 adapter 自己排队并设置上限。

notification 只说明变化发生，不自动构成完整当前状态。可靠状态必须使用以下一种协议：

- start response 返回 snapshot/revision，并规定从哪个 sequence 继续；
- snapshot request 与带 sequence 的 notification 组合；
- 协议保证注册路由早于 start，并允许 start 期间缓存事件。

不能依赖“先请求快照还是先监听”的时序巧合消除事件缺口。

## 显式资源

持续资源使用 client-generated resource ID，使 start 发出前就能建立路由和停止目标：

```text
创建 resource ID
  → 注册 notification 路由
  → start(resource ID, params)
  → 接收 notification(resource ID, ...)
  → completed/exited 或 stop(resource ID)
  → 删除路由
```

- 后端 key 至少包含 connection ID 和 resource ID，不同 connection 不能相互停止资源。
- 同一 connection 的重复 resource ID 返回稳定错误，不能覆盖旧资源。
- stop response 返回前必须保证之后不再发送该资源的 notification。
- connection 关闭释放该 connection 创建的全部资源。
- Main 按 renderer attachment 记录资源；renderer 断开只停止它拥有的资源。
- 多个本地 listener 共享同一后端资源时，最后一个 listener dispose 后才发送 stop。
- start 失败、stop 失败、终止 notification 和 connection close 都必须结束前端事件并删除路由。

## 取消

renderer IPC 的 `CancellationToken` 只取消 renderer ↔ Main 的 call。后端没有通用 client request cancellation，因此 connection 的 `request` 不接收 token。

领域 adapter 按实际协议处理：

| 操作 | 正确取消 |
| --- | --- |
| 正在运行的 turn | 已取得 `threadId`/`turnId` 后发送 `turn/interrupt` |
| 已启动的 command | 使用稳定 process/command ID 发送 terminate |
| 文件或其他 watch | 使用 resource ID 发送 unwatch/stop |
| 正在进行的 login | 使用对应 login ID 发送 login cancel |
| 没有 cancel method 的普通 request | 不承诺取消后端工作；最多让 renderer 停止等待，并继续在 Main 配对迟到 response |

需要新增取消语义时，协议必须先具备稳定 operation ID 和 typed cancel request。adapter 监听 token，发送 cancel，并在完成、取消或 close 后释放 token listener。协议还要定义 cancel 与正常完成竞争时的唯一终态。

丢弃 Promise、从 pending map 提前删除、忽略迟到 response 或发送未知 method 都不构成取消。即使 renderer 已停止等待，connection 仍必须消费该 response，避免 pending 泄漏或把迟到 response 误判成新 connection 消息。

## Server request

Rust 需要审批、用户输入、权限确认或宿主能力时发送 typed server request。connection 按生成 method 注册 handler，并保证每个 request 恰好回复一次成功或错误。

| handler 所需能力 | owner |
| --- | --- |
| Main 已拥有的系统或进程能力 | `src/platform/<domain>/electron-main/` handler |
| renderer 才能完成的交互 | 独立 host channel → 明确的 `src/workbench/services/<domain>/` service |
| Rust 自己可以完成的领域行为 | 不发送 server request，直接调用 Rust 领域能力 |

handler 先按 thread、turn、resource 或 session 查找 renderer owner。未知 method 返回 method-not-found；无 owner、窗口关闭、handler dispose、超时和 connection close 返回稳定错误。不能广播请求，也不能让 server request 永久占用 pending 表。

## 顺序与并发

顺序由 Rust 协议的 serialization scope 决定，常见 key 包括 thread、path、process、watch 和 session。只属于 connection 的资源 key 同时包含 connection ID；需要跨 connection 共享的资源使用真实全局 key。

- 同 key 的修改按协议定义串行。
- 明确允许共享读的 request 可以并行，但不能越过已排队的写。
- 不同 key 默认并行；共享 connection 不等于全局串行。
- 前端不得增加第二套领域锁修补后端顺序。

业务正确性依赖顺序时，serialization scope、revision 或领域锁必须位于 Rust owner；前端调用先后不构成线上保证。

## 错误与重试

| 层 | 例子 | owner 行为 |
| --- | --- | --- |
| transport | EOF、进程退出、写失败、队列满 | close connection 并拒绝 pending |
| protocol | method 不存在、params 无效、未 initialize | connection 解码为 protocol error |
| domain | 文件不存在、状态冲突、权限拒绝 | Main 领域 channel 转为领域错误 |
| decode | response 与生成类型不匹配 | 关闭或标记 connection 失效，报告版本/实现错误 |

错误分类依赖稳定 numeric code 和结构化 data，不匹配英文 message。若调用方必须区分的领域错误没有结构化 data，应先补协议；不能在 adapter 猜测。JSON-RPC envelope、stdout 内容和 Rust 内部错误不能进入 renderer。

输入队列饱和产生的 overload error 可以按协议标为可重试，但只读 request 才能由调用层使用指数退避和抖动重试。已写入 transport 的修改在断线后属于未知结果，除非协议提供幂等 ID 和查询确认，否则不自动重试。

## 初始化和关闭

每条 connection 只执行一次：

1. 打开 transport；
2. 发送 typed `initialize` request；
3. 校验响应并记录运行环境信息；
4. 发送 `initialized` notification；
5. 进入 `Ready` 并开放领域 request。

其他 request 在握手前会被拒绝。初始化期间到达的已知 notification/server request 继续读取并按原顺序暂存，未知 server request 立即拒绝，不能阻塞 stdout reader。

协议生成物与打包的后端可执行文件必须来自同一版本。`initialize` 返回的 user agent 只用于诊断，不是协议版本协商；若允许连接独立升级的后端，必须先在协议中增加明确的兼容性字段，不能从字符串推断兼容性。

关闭时按固定顺序：

1. 停止接受新领域 request；
2. 记录单一关闭原因并停止 reader/writer；
3. 拒绝所有 client pending；
4. 结束所有 server request handler 并回复可发送的关闭错误；
5. 终止资源路由和 notification listener；
6. 通知领域 adapter 产生契约允许的终止事件或状态变化。

新 connection 重新握手并使用新代次。旧 ID、pending、response 和 notification 不进入新代次；资源是否重建由领域明确决定。

## 背压与输出纪律

transport 的输入和输出队列必须有界。写入尊重 stream drain；大 payload 使用协议定义的分块或资源 notification。stdout 只写线上 JSONL，日志和诊断写 stderr。不能以无限队列、无限 listener 缓冲或阻塞 reader 吸收压力。
