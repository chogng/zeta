# 实现流程

本文件用于新增、修改或 review 一项完整 API。`<domain>` 和 `<action>` 必须替换成真实领域词；只创建具有独立职责和真实调用方的文件。

## 1. 固定前端领域契约

先在以下一个 owner 定义 renderer 真正需要的接口：

```text
src/platform/<domain>/common/<domain>.ts
src/workbench/services/<domain>/common/<domain>.ts
```

契约必须明确：

- input、result 和前端领域错误；
- event payload、初始状态和事件缺口处理；
- 哪个方法创建长期资源，谁拥有 dispose；
- 用户“停止”是中断后端工作，还是只停止本地等待；
- renderer、session 和 connection 关闭后调用方看到什么。

领域名和方法名按前端行为命名，不复用线上 method。若当前接口暴露线上 DTO、transport state、request ID 或错误 envelope，先收敛最终接口并迁移调用方，不为错误接口设计 adapter。

## 2. 固定 session 和 renderer 归属

在写线上调用前先回答：

- session key 由工作区执行环境、远端目标、账户和权限边界中的哪些字段组成；
- 多个 renderer 是否共享同一 session；
- 哪个 renderer 拥有新建的 thread、turn、resource 或交互请求；
- renderer 断开时释放哪些 attachment 资源，何时停止后端 session；
- server request 如何从线上标识找到唯一 renderer。

这些状态由 `src/platform/appServer/electron-main/appServerProcessService.ts` 拥有。领域 channel 必须使用 IPC `context` 访问 attachment，不能忽略 context 或以全局 active window 代替。

## 3. 选择消息、标识和顺序

按 [协议语义](protocol-semantics.md) 选择 request、notification、显式资源或 server request。

持续资源先设计：

- client-generated resource ID；
- start request/response；
- 带 ID 的 notification 和终止事件；
- stop request/response；
- renderer disconnect 和 connection close 的清理。

可取消操作先设计 operation ID 和 typed cancel request。同步写出 serialization scope；没有领域理由时不同 key 并行，不能为实现方便全局串行。

## 4. 定义并生成 Rust 协议

领域 DTO 放在：

```text
../app-server-protocol/src/protocol/v2/<domain>.rs
```

统一注册点绑定 method、params、response 和 serialization scope：

```rust
<DomainAction> => "<domain>/<action>" {
    params: v2::<DomainActionParams>,
    serialization: <scope>,
    response: v2::<DomainActionResponse>,
}
```

事件加入 typed server notification registry；Rust 等待宿主回复时加入 typed server request registry。需要前端分类的错误使用稳定 numeric code 和结构化 data。

生成输出由以下目录拥有：

```text
../app-server-protocol/schema/typescript/
```

生成器必须产出并验证：

- request method → params → response map；
- notification method → params map；
- server request method → params → response map；
- envelope、request ID、错误和初始化 DTO；
- JSON Schema。

若生成结果只有 union 和分散 response 类型，停止前端实现，先修改 `../app-server-protocol/src/export.rs` 或实际生成 owner。不能在前端手写 method、response map 或重复 DTO。

## 5. 接入 Rust processor

领域 processor 放在：

```text
../app-server/src/request_processors/<domain>_processor.rs
```

processor 只负责：

- 接收生成协议类型；
- 机械转换为 Rust 领域值；
- 调用领域能力；
- 把结果或稳定错误转成 response。

业务校验、持久化、系统访问和长期状态属于实际领域能力。确有跨请求资源时才增加：

```text
../app-server/src/<domain>_resource.rs
```

资源 key 至少包含 `(connection_id, resource_id)`。在 `../app-server/src/message_processor.rs` 的唯一分派中接入 processor；不建立第二个 dispatcher，也不把领域实现堆进统一分派文件。

## 6. 实现共享 connection

环境无关的 connection 放在：

```text
src/platform/appServer/common/appServerConnection.ts
```

它依赖一个消息 transport 接口并消费生成 map：

```ts
interface IAppServerConnection {
	readonly state: AppServerConnectionState;
	readonly onDidChangeState: Event<AppServerConnectionState>;

	request<M extends keyof AppServerRequestMap>(
		method: M,
		params: AppServerRequestMap[M]['params'],
	): Promise<AppServerRequestMap[M]['response']>;

	onNotification<M extends keyof AppServerNotificationMap>(
		method: M,
		listener: (params: AppServerNotificationMap[M]) => void,
	): IDisposable;

	registerServerRequestHandler<M extends keyof AppServerServerRequestMap>(
		method: M,
		handler: (params: AppServerServerRequestMap[M]['params']) => Promise<AppServerServerRequestMap[M]['response']>,
	): IDisposable;
}
```

connection 拥有唯一 request ID allocator、client pending、server pending、初始化状态、typed message classification 和 close。它不枚举领域 method，不暴露 raw message 或 `unknown` 调用，`request` 也不接收 `CancellationToken`。

## 7. 实现 stdio transport 与进程服务

文件位置：

```text
src/platform/appServer/node/appServerStdioTransport.ts
src/platform/appServer/electron-main/appServerProcessService.ts
```

stdio transport 只处理 JSONL framing、stdin/stdout、stderr、drain、队列上限和 close。stdout 只允许线上消息；解析错误携带有限诊断并使 connection 进入明确失败状态。

进程服务负责 executable resolution、spawn、session key、transport/connection 创建、renderer attachment 和应用关闭。它把 exit、signal、stdin error、stdout EOF 收敛成一次 close，不建立并行重连循环或隐藏备用 transport。

## 8. 编写 Main 领域 channel

文件位置：

```text
src/platform/<domain>/electron-main/<domain>AppServerChannel.ts
```

只属于 workbench 的领域改放 `src/workbench/services/<domain>/electron-main/`。领域 channel 实现 `IServerChannel<RendererContext>`，使用 context 取得正确 connection：

```ts
export class DomainAppServerChannel implements IServerChannel<RendererContext> {
	public constructor(private readonly processService: IAppServerProcessService) {}

	public call<T>(context: RendererContext, command: string, arg: unknown, token: CancellationToken): Promise<T> {
		const connection = this.processService.getConnection(context);
		switch (command) {
			case DomainIpcCommand.Action:
				return this.action(connection, arg, token) as Promise<T>;
			default:
				throw new Error(`Unknown domain command: ${command}`);
		}
	}
}
```

私有 action 使用生成 method/DTO，返回前端领域类型。允许转换 URI/path、时间、枚举、错误和资源 ID；禁止权限决定、持久化、业务缓存和静默重试。

如果领域操作有后端 cancel，action 监听 token 并发送 typed cancel request，完成路径释放 listener。没有后端 cancel 的领域方法不应在公开契约中承诺取消；即使 IPC call 被 renderer 放弃，Main 仍保留线上 pending 直到 response 或 close。

## 9. 编写 renderer channel client

IPC command/event 和 client 放在领域目录：

```text
src/platform/<domain>/common/<domain>Ipc.ts
```

```ts
export class DomainChannelClient implements IDomainService {
	public constructor(private readonly channel: IChannel) {}

	public action(input: DomainInput): Promise<DomainResult> {
		return this.channel.call(DomainIpcCommand.Action, input);
	}
}
```

需要恢复 URI、Error 或前端对象时在 client 中完成。该文件只依赖 `IChannel` 和前端领域类型，不导入 `../app-server-protocol/schema/typescript/`。

## 10. 事件和资源

connection 级 notification 只解析一次。领域 channel 订阅 typed notification，再转换成前端 `Event`。

资源 adapter 按顺序执行：

1. 首个 renderer listener 创建 resource ID 并在 start 前注册路由；
2. start 期间缓存提前到达的 notification；
3. start response 与 revision/snapshot 合并后开放事件；
4. notification 按 resource ID 进入正确 emitter；
5. 最后一个 listener dispose、renderer disconnect、终止事件或 connection close 触发清理；
6. 需要停止后端时发送 stop，并等待“response 后无新事件”的协议保证。

不能在 renderer 直接订阅 connection notification，也不能让多个领域 adapter 各自解析同一线上事件流。

## 11. 实现 server request handler

Main 可直接完成的 handler 放在：

```text
src/platform/<domain>/electron-main/<domain>HostRequestHandler.ts
```

需要 renderer 交互时，在对应 workbench service 中定义 host channel，并由 Main 根据 attachment 路由到唯一 renderer。线上 `ServerRequest` union 不进入 renderer。

handler 必须：

- 使用生成 params/response；
- 通过 thread、turn、resource 或 session 找 owner；
- 对未知、无 owner、超时、窗口关闭、dispose 和 connection close 返回稳定错误；
- 每个 request 恰好回复一次；
- 把权限与业务决定交给真正 owner。

## 12. 产品装配

在 `src/code/electron-main/app.ts` 创建一次进程服务并注册领域 channel：

```ts
server.registerChannel(
	DOMAIN_CHANNEL_NAME,
	new DomainAppServerChannel(processService),
);
```

renderer 在对应 `electron-browser` 文件把 `DomainChannelClient` 注册为领域 service。产品装配文件不包含 DTO 转换、领域 method switch、session 业务状态或 protocol parser。

## 13. 让错误 owner 退场

同批迁移所有调用方并删除以下重复所有权；文件删除仍遵守仓库的单独确认规则：

- renderer 中的 process spawn、stdout reader、request ID 和 pending map；
- 领域目录中的通用 JSON-RPC parser 或 initialize 状态；
- 手写线上 DTO、method 常量和 response map；
- 绕过领域接口的 `invoke`、`sendRequest` 或 raw notification 入口；
- 按领域启动的重复 app-server 进程；
- 依赖 active window 的 server request 广播；
- 旧后端、备用 transport、双写和断线重放路径。

## 14. 测试

| 层 | 必测行为 |
| --- | --- |
| protocol generator | method/params/response maps、notification/server request maps、生成物无 diff |
| Rust processor | DTO 转换、领域调用、结构化错误、serialization scope |
| resource manager | connection 隔离、重复 ID、stop 后无事件、connection cleanup |
| TypeScript connection | initialize gate、双向 ID 分离、request pairing、未知/重复 response、notification、server request、close |
| stdio transport | 分行/粘连输入、无效 JSON、stderr 隔离、drain、队列上限、EOF |
| process service | session 复用与隔离、renderer attach/detach、进程退出只关闭一次 |
| Main 领域 channel | context 选 session、前端↔线上转换、领域取消、错误、事件和 dispose |
| renderer client | 原领域接口、对象恢复和 listener 生命周期 |
| 双端 | initialize/initialized、真实 request、持续事件、cancel/stop、server request response、renderer 断开和进程退出 |

## Review 检查

使用 `rg` 和定向测试确认：

- 生成 DTO 只出现在 `platform/appServer/{common,node}` 和领域 `electron-main` adapter/handler；
- 线上 method string 只来自生成物；
- 前端只有一个 request ID allocator、client pending、server pending、message classifier 和 transport reader；
- 每个领域 channel 使用 renderer context，不读取全局 active window；
- 每个长期资源都有 start、终止、stop、renderer cleanup 和 connection cleanup；
- 只有协议支持取消的操作才把 token 映射到后端；迟到 response 仍被消费；
- `app.ts` 只装配，`message_processor.rs` 只统一分派；
- 没有通用业务 API、`unknown` DTO、错误消息匹配、旧后端、备用路径或静默重放。
