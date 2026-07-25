# Zeta Desktop 架构与协作边界

> 负责人：Desktop 开发者  
> Rust 对接负责人：zeta-rs 开发者  
> 当前开发基线：[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md)

## 1. 目标

Desktop 是 Zeta 的 Electron 富客户端，负责窗口、浏览器、系统能力和 UI，不拥有 Agent、Thread、Turn、Item、审批策略或持久化状态机。

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
- Browser View、Tab、Session、CDP 和下载；
- Renderer 纯 UI 状态与服务端状态投影；
- 宿主权限、导航策略、origin 策略；
- Desktop 端集成测试。

Desktop 不负责：

- Thread、Turn、Item、Tool Call 的权威状态；
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
│   └── app-server/v1/
├── package.json
└── tsconfig.json
```

`desktop/generated/` 由 zeta-rs 协议生成命令更新，不手写 wire DTO。

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

## 5. Preload API

Preload API 必须是领域化、强类型、可枚举的接口：

```ts
interface ZetaDesktopApi {
  thread: {
    start(params: ThreadStartParams): Promise<ThreadStartResult>;
    read(params: ThreadReadParams): Promise<ThreadReadResult>;
    resume(params: ThreadResumeParams): Promise<ThreadReadResult>;
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

Renderer 不复制 Rust 状态机。遇到 durable `sequence` 或流式 `streamSeq` 空洞时，停止合并
当前实体，并通过 `thread/read` 或 `thread/resume` 获取权威 snapshot。

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
[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md) 为准。

## 9. Rust 交付给 Desktop 的产物

每次协议交付至少包含：

- 可运行的 `zeta` 二进制；
- `zeta app-server --listen stdio://`；
- `desktop/generated/app-server/v1/types.ts`；
- `schemas/app-server/v1.schema.json`；
- schema hash；
- 当前版本和前一兼容版本 fixtures；
- Rust contract tests；
- API 变更说明。

## 10. Desktop 验收

Desktop 完成的最低证据：

- TypeScript strict build 通过；
- initialize 成功并校验 schema hash；
- Thread 创建、读取、恢复和 Turn 中断端到端通过；
- 通知能从 App Server 到 Renderer；
- 未生成或参数错误的 IPC 被拒绝；
- 不可信网页无法访问应用 IPC；
- Browser Target 关闭后不会操作其他 Tab；
- App Server 崩溃、重启和 graceful shutdown 有测试。
