# 实现流程

本文件用于新增、修改或 review 一条完整 API。示例中的 `<domain>` 和 `<action>` 必须替换成真实领域词；不要为了匹配模板创建没有调用方的文件。

## 1. 先固定最终领域契约

在 `src/platform/<domain>/common/<domain>.ts` 或 `src/workbench/services/<domain>/common/` 定义 renderer 真正需要的接口。命名按前端领域，不复用 Rust method 名。

契约至少明确：

- method 的 input、result 和领域错误；
- event 的 payload、初始状态和事件缺口处理；
- `CancellationToken` 实际取消什么；
- listener、resource 和 service 的 dispose 行为；
- connection 关闭后调用方看到什么。

若当前接口已经泄漏线上 DTO、transport state 或错误 envelope，先改最终接口，再做对接。不要为保留错误接口设计 adapter。

## 2. 选择消息和标识

使用 [协议语义](protocol-semantics.md) 的消息表。对持续资源先设计 client-generated resource ID、start response、notification、终止 notification 和 stop request；对可取消操作先设计 operation ID 和 cancel request。

同时写出 serialization scope。没有领域理由时使用并行；不能因为实现方便使用全局串行。

## 3. 定义 Rust protocol

领域 DTO 放在：

```text
../app-server-protocol/src/protocol/v2/<domain>.rs
```

统一注册点绑定 method、params、response 和 scope：

```rust
<DomainAction> => "<domain>/<action>" {
    params: v2::<DomainActionParams>,
    serialization: <scope>,
    response: v2::<DomainActionResponse>,
}
```

事件加入 typed server notification registry；Rust 等待前端回复时加入 typed server request registry。线上错误使用稳定 code 和结构化 data。

协议 crate 同步生成并验证：

- request method → params → response map；
- notification method → params map；
- server request method → params → response map；
- JSON schema 和 TypeScript DTO。

生成输出固定在 `../app-server-protocol/schema/typescript/`。若目标类型或映射缺失，修改协议导出逻辑，不在前端补类型。

## 4. 接入 Rust processor

领域 processor 放在：

```text
../app-server/src/request_processors/<domain>_processor.rs
```

processor 负责：

- 接收生成协议类型；
- 做协议值到 Rust 领域值的机械转换；
- 调用领域能力；
- 把领域结果或稳定错误转为 response。

业务校验、持久化、系统访问和长期状态属于实际领域能力。跨请求资源需要 manager 时放在 `../app-server/src/<domain>_resource.rs`，并以 `(connection_id, resource_id)` 作为 owner key。

在 `../app-server/src/message_processor.rs` 的唯一 match 中分派到 processor。不要创建第二个 dispatcher；`message_processor.rs` 也不能吸收领域实现。

## 5. 更新共享 TypeScript connection

连接机制放在：

```text
src/platform/appServer/node/appServerTransport.ts
src/platform/appServer/node/appServerConnection.ts
src/platform/appServer/electron-main/appServerProcessService.ts
```

只有新增线上机制时才修改 connection。例如生成映射增加普通 method 时，connection 通常不需要改动；新增 server request 分类、初始化 capability 或 transport framing 时才需要改。

connection 对领域 adapter 暴露的能力保持机械：

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
		handler: (params: AppServerServerRequestMap[M]['params']) =>
			Promise<AppServerServerRequestMap[M]['response']>,
	): IDisposable;
}
```

三个 map 都是生成物。接口不枚举领域方法，不暴露 request ID、transport、pending map、raw message 或任意 `unknown` 调用。

## 6. 编写 Electron Main 领域 channel

文件位置：

```text
src/platform/<domain>/electron-main/<domain>AppServerChannel.ts
```

若领域只属于 workbench service，放到对应 `src/workbench/services/<domain>/electron-main/`。不要把领域 channel 放进 `platform/appServer`。

channel 实现 `IServerChannel`，它是两份契约的转换点：

```ts
export class DomainAppServerChannel implements IServerChannel {
	constructor(private readonly connection: IAppServerConnection) {}

	call<T>(
		_context: string,
		command: string,
		arg: unknown,
		cancellationToken: CancellationToken,
	): Promise<T> {
		switch (command) {
			case DomainIpcCommand.Action:
				return this.action(arg, cancellationToken) as Promise<T>;
			default:
				throw new Error(`Unknown domain command: ${command}`);
		}
	}
}
```

私有 `action` 使用生成 method 和 DTO，转换后只返回前端领域类型。channel 可以做 URI/path、时间、枚举、错误和资源 ID 转换；不能做权限决定、持久化、业务缓存或静默重试。

取消不能直接传给通用 `connection.request`。有取消语义时，channel 监听 token 并发送领域 typed cancel request；start 和 cancel 都引用同一 client-generated operation ID。完成路径必须释放 token listener，并对 cancel request 的失败作明确处理。

## 7. 编写 renderer channel client

领域 IPC 名称和 client 放在：

```text
src/platform/<domain>/common/<domain>Ipc.ts
```

client 只依赖 `IChannel` 和前端领域类型：

```ts
export class DomainChannelClient implements IDomainService {
	constructor(private readonly channel: IChannel) {}

	action(input: DomainInput, token: CancellationToken): Promise<DomainResult> {
		return this.channel.call(DomainIpcCommand.Action, input, token);
	}
}
```

需要恢复 URI、Error 或前端对象时在 client 中完成。这里不得导入 `../app-server-protocol/schema/typescript/`。

## 8. 事件和资源

connection 级 notification 在 connection 中只解析一次。领域 channel 订阅所需 typed notification，再转换成前端 `Event`。

资源事件遵循：

1. 首个 renderer listener 创建 resource ID、注册本地路由并发送 start；
2. 后续 listener 复用同一资源；
3. notification 按 resource ID 进入正确 emitter；
4. 最后一个 listener dispose 后发送 stop；
5. stop response、终止 notification 或 connection 关闭后释放路由和 emitter。

如果 start response 包含初始 snapshot/revision，先缓存 start 期间到达的 notification，再按 revision 合并。不能留下 snapshot 与事件之间的空窗。

## 9. Server request handler

Electron Main 能直接完成的 server request handler 与领域 channel 放在同一领域 `electron-main` 目录，由 connection registry 注册。需要 renderer 交互时，定义独立前端 host service/channel；不要把线上 `ServerRequest` 对象广播给 workbench。

handler 必须：

- 使用生成 params/response；
- 对未知、超时、window 关闭和 handler dispose 返回稳定错误；
- 每个 request 恰好回复一次；
- 把权限与业务决定交给真正 owner。

## 10. 装配

在 `src/code/electron-main/app.ts` 创建一次进程 owner 和共享 connection，再注册领域 channel：

```ts
server.registerChannel(
	DOMAIN_CHANNEL_NAME,
	new DomainAppServerChannel(connection),
);
```

renderer 在对应 `electron-browser` 文件把 `DomainChannelClient` 注册为领域 service。产品装配文件不包含 DTO 转换或领域 method switch。

## 11. 测试

| 层 | 必测行为 |
| --- | --- |
| protocol | method、params、response、notification、server request 和生成文件同步 |
| Rust processor | DTO 转换、领域调用、稳定错误、serialization scope |
| resource manager | connection 隔离、重复 ID、stop 后无事件、connection cleanup |
| TypeScript connection | initialize gate、request pairing、未知 response、通知、server request、关闭和背压 |
| Electron Main channel | 前端↔线上转换、取消、错误、事件和 dispose |
| renderer client | 原领域接口、对象恢复和 listener 生命周期 |
| 双端 | initialize、真实 request、持续事件、cancel/stop、server request response 和进程退出 |

## Review 检查

使用 `rg` 检查：

- 生成 DTO 只出现在 `platform/appServer/node` 和领域 `electron-main` adapter；
- 线上 method string 只来自生成物；
- 全部前端只有一个 request ID allocator、pending map、message parser 和 transport reader；
- 每个 resource 有 start、终止、stop 和 connection cleanup；
- `CancellationToken` 没有被丢弃或伪装成后端取消；
- `app.ts` 只装配，`message_processor.rs` 只统一分派；
- 没有通用业务 API、`unknown` DTO、旧后端回退或断线静默重放。
