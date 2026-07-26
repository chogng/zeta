# Zeta Desktop 架构与协作边界

> 负责人：Desktop 开发者  
> Rust 对接负责人：zeta-rs 开发者  
> 当前开发基线：[`zeta-app-server-api.md`](zeta-app-server-api.md)

## 1. 目标

Desktop 是 Zeta 的 Electron 富客户端，负责窗口、浏览器、系统能力和 UI，不拥有
Session、Thread、Turn、ThreadItem、审批策略或持久化状态机。

Desktop 只能通过版本化 App Server API 使用 zeta-rs：

```text
Renderer
  → typed Preload API
  → Electron Main
  → JSON-RPC / JSONL / stdio
  → zeta app-server
```

Desktop 禁止执行 `zeta ask ...` 后解析终端输出，也禁止直接链接 `zeta-core`。

## 2. Desktop 所有权

Desktop 负责：

- Electron Main、Preload、Renderer；
- App Server 进程启动、初始化、监督、重启和关闭；
- 窗口、菜单、快捷键、命令面板；
- Browser View、Tab、BrowserSession、CDP 和下载；
- Renderer 纯 UI 状态与服务端状态投影；
- 宿主权限、导航策略、origin 策略；
- Desktop 端集成测试。

Desktop 不负责：

- Session、Thread、Turn、ThreadItem、Tool Call 的权威状态；
- Agent 规划和工具循环；
- 是否需要审批的业务策略；
- rollout、SQLite 投影和 Thread writer lease；
- 模型供应商与长期凭据持久化；
- Rust 协议 DTO 的定义。

## 3. 目录边界

```text
desktop/
├── src/
│   ├── main/
│   │   ├── app-server/
│   │   ├── browser/
│   │   ├── ipc/
│   │   ├── security/
│   │   └── windows/
│   ├── preload/
│   └── renderer/
├── generated/
│   └── app-server/
├── package.json
└── tsconfig.json
```

`desktop/generated/` 由 zeta-rs 协议生成命令更新，不手写 wire DTO。
生成的 `APP_SERVER_SCHEMA_HASH` 是 bundled Desktop 的 exact-schema 基线；Electron Main
必须比较 initialize response，hash 不一致时不得创建业务窗口或进入 Ready。

## 4. Main Process

Main 必须：

1. 从应用包内确定的绝对路径启动 `zeta app-server --listen stdio://`；
2. 使用 `shell: false`，只传递环境变量 allowlist；
3. 在创建业务 UI 前完成 `initialize`；
4. 校验 protocol version、schema hash 和 server build；
5. 将 stdout 仅交给 JSONL 协议解析器；
6. 对 stderr 做大小限制和 secret 脱敏；
7. 为启动、初始化、请求和关闭设置 deadline；
8. 采用有上限的指数退避处理崩溃重启；
9. 校验每个 Renderer IPC 的 sender、frame URL、origin 和参数；
10. 持有 Browser Target 与 Resource 的宿主侧所有权。

Main 不把 `ipcRenderer`、`fs`、`child_process`、`webContents` 或任意 JSON-RPC method
直接暴露给 Renderer。

当前 `ChildProcessJsonlTransport` 将子进程 stream lifecycle 与 JSON-RPC pairing 分开。它在
积累无限 buffer 前按原始 byte 拒绝超过 1 MiB 的 frame，只接受严格 LF 和有效 UTF-8；
outbound write 同时等待 callback 与 drain，并限制 pending write 数。child/stdio 任一错误
都会关闭 transport；stderr 只保留 64 KiB ring，诊断读取时脱敏 credential。`close()` 异步、
幂等，并在 graceful deadline 后强制终止。`npm run test:main` 覆盖分片 UTF-8、超限 frame、
非法 framing、backpressure、stderr 和 close。

`JsonRpcPeer` 在 transport 之上负责双向 JSON-RPC envelope、request ID pairing、remote
error、timeout/abort、late/unknown/duplicate response、入站 handler cancellation、pending
上限和 listener 隔离。协议生成器输出 `APP_SERVER_METHODS` 与
`APP_SERVER_NOTIFICATIONS` typed definitions，Electron Main 通过 `AppServerClient` 使用；
产品代码不能传任意 method string 或手写 result 泛型。

`AppServerSession` 独占一个 peer，只有 initialize response 同时通过 server identity、
protocol version 和 schema hash gate 后才进入 Ready，并保存协商后的 server
info/capabilities。

`AppServerSession` 是 connection lifecycle。它不是 canonical 产品 `Session`，不得保存
产品 Session membership、lineage 或权威业务状态；Renderer 只维护可丢弃并可 resync 的
Session/Thread projection。
`AppServerSupervisor` 只接受绝对 executable、显式 child environment allowlist，并管理
Stopped/Starting/Initializing/Ready/Stopping/Crashed/Restarting 状态、initialize deadline、
有界指数退避和 crash budget。崩溃会拒绝旧 Session 的 pending request；新 Session 不自动
重放结果未知的副作用操作。

结构化 IPC router 集中注册有限 channel，并在调用 validator/handler 前同时验证目标
webContents、main frame identity 和确切入口 URL。当前窄 IPC surface 对 params 做
exact-shape validation；unknown field、错误 enum、空 ID 或畸形 Turn input 均不会到达
App Server。协议生成 runtime validator 后，应替换这些同形显式 validator 的来源而不改变
router 边界。

## 5. Preload API

Preload API 必须是领域化、强类型、可枚举的接口：

```ts
interface ZetaDesktopApi {
  appServer: {
    getConnectionState(): Promise<AppServerConnectionState>;
    onConnectionState(listener: (state: AppServerConnectionState) => void): () => void;
  };
  session: {
    create(params: SessionCreateParams): Promise<SessionResult>;
    read(params: SessionReadParams): Promise<SessionResult>;
    list(): Promise<SessionListResult>;
    subscribe(params: SessionSubscribeParams): Promise<SessionSubscribeResult>;
    createThread(params: SessionThreadCreateParams): Promise<SessionThreadResult>;
    forkThread(params: SessionThreadForkParams): Promise<SessionThreadResult>;
  };
  thread: {
    read(params: ThreadReadParams): Promise<ThreadReadResult>;
    subscribe(params: ThreadSubscribeParams): Promise<ThreadSubscribeResult>;
    unsubscribe(params: ThreadUnsubscribeParams): Promise<void>;
  };
  turn: {
    start(params: TurnStartParams): Promise<TurnStartResult>;
    interrupt(params: TurnInterruptParams): Promise<void>;
  };
  events: {
    subscribe(listener: (event: DesktopEvent) => void): () => void;
  };
}
```

禁止提供：

```ts
execute(method: string, params?: unknown): Promise<unknown>
```

## 6. Renderer

Renderer 负责 Command Registry、路由、组件、输入框、虚拟列表和状态投影。

```text
button / menu / shortcut
  → UI Command
  → typed preload method
  → domain RPC
```

Renderer 不复制 Rust 状态机。遇到 durable `sequence` 或 `streamCursor` 空洞时，停止合并
当前实体，并通过 `session/subscribe` 或 `thread/subscribe` 获取权威 snapshot + gap。

## 7. Browser Capability

Electron Main 是 Browser Target 的唯一权威持有者。

Desktop 对 Rust 暴露语义动作：

- `browser/observe`
- `browser/perform`
- `browser/getPdf`

不能暴露任意 CDP method。每个 `targetId` 必须：

- 绑定创建它的 App Server connection；
- 在 Tool Call 开始前固定；
- 关闭后返回 `BrowserTargetUnavailable`；
- 不得静默切换到另一个活动 Tab。

第三方网页必须使用：

```text
nodeIntegration: false
contextIsolation: true
sandbox: true
无特权 preload
无应用 IPC
独立 session / partition
```

## 8. Desktop 提交 App Server 能力需求

Desktop 开发者在实现前提交一份符合
[`zeta-api-interface-requirements.md`](zeta-api-interface-requirements.md) 的产品接口需求。
Desktop 是需求提出方；zeta-rs 是已接受 App Server 契约的 owner。接口必须同时评估 CLI、
daemon 和远程客户端影响，不能定义为 Desktop 私有业务 API。

文档必须覆盖：

- Client → Server 方法；
- Server → Client 请求；
- Server → Client 通知；
- Resource RPC；
- Browser Target 生命周期；
- 错误码、超时、取消、幂等和顺序；
- 每个请求、成功响应和错误响应的 JSON fixture。

zeta-rs 开发者根据该文档实现 Rust DTO、dispatcher、typed client、handler、schema 和
TypeScript 生成。进程内 CLI client 与 Desktop stdio client 必须经过同一个 dispatcher。

当前已接受的方法、通知、错误码和前端可开发范围以
[`zeta-app-server-api.md`](zeta-app-server-api.md) 为准。

## 9. Rust 交付给 Desktop 的产物

每次协议交付至少包含：

- 可运行的 `zeta` 二进制；
- `zeta app-server --listen stdio://`；
- `zeta-rs/app-server-protocol/schema/types.ts`；
- `zeta-rs/app-server-protocol/schema/schema.json`；
- schema hash；
- 当前 schema fixtures；
- Rust contract tests；
- API 变更说明。

## 10. Desktop 验收

Desktop 完成的最低证据：

- TypeScript strict build 通过；
- initialize 成功并校验 schema hash；
- Session 创建、Thread 创建/fork、订阅恢复和 Turn 中断端到端通过；
- 通知能从 App Server 到 Renderer；
- 未生成或参数错误的 IPC 被拒绝；
- 不可信网页无法访问应用 IPC；
- Browser Target 关闭后不会操作其他 Tab；
- App Server 崩溃、重启和 graceful shutdown 有测试。
