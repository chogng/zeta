# 连接与文件放置

设计进程、连接、IPC、握手、重连，或决定连接相关文件位置时，完整读取本文件。

## 两段连接

renderer 到 Rust 不是一条直接连接，而是两段职责不同的边界：

| 边界 | 线上机制 | 对外形态 | owner |
| --- | --- | --- | --- |
| renderer ↔ Electron Main | VS Code IPC | 多个领域 channel / service | 各领域 adapter 与 Electron Main 注册 |
| Electron Main ↔ Rust app-server | app-server 协议与 JSONL 等传输 | 一条承载多个领域方法的连接 | Electron Main 连接 owner 与 Rust transport |

```text
renderer 的 VS Code 领域服务
  → 领域 IChannel
  → Electron Main 领域 IServerChannel adapter
  → 共享 Rust app-server client
  → stdio / socket 帧
  → Rust 协议分派与领域能力
```

renderer 不持有通用 JSON-RPC client，也不按线上方法名调用 Rust。确实需要展示后端连接状态时，只暴露小型的生命周期或状态领域服务，不暴露线上连接对象。

## 连接生命周期

- Electron Main 为一次“前端 host 到后端目标”的会话创建并拥有 Rust 进程和连接。同一连接承载文件、终端、搜索等多个领域。
- 先完成 initialize、版本和能力协商，再允许领域 channel 进入 ready 状态。
- 连接关闭时，统一拒绝全部待处理请求，释放连接级资源，并让领域 adapter 收到明确的终止原因。
- 重连建立新连接代次，重新 initialize。旧连接的通知和响应不得进入新代次。
- 只有协议明确支持恢复的资源才可恢复；watch、流和会话不能凭 TypeScript 本地对象假装仍然有效。

不要把“整个应用只有一条永久连接”写死。多窗口、多后端目标或不同权限边界可以产生多次独立会话，但每次会话内不得为每个领域重复建连接。

## 传输职责

| 职责 | 位置 |
| --- | --- |
| 服务端帧格式、解析、大小限制和写出规则 | `zeta-rs/app-server-transport` |
| Rust 请求、响应、通知和错误类型 | `zeta-rs/app-server-protocol` |
| 可复用的 TypeScript client 侧子进程、stdio、Buffer 与帧读写机制 | 确有多个调用方时放在 `zeta-ts/src/zeta/platform/app-server/node` |
| Electron 进程创建、监督、退出处理和 IPC 注册 | `zeta-ts/src/zeta/platform/app-server/electron-main` |

TypeScript client 侧必须按 Rust transport 的线上规则读写，但不能在领域 adapter 中各自实现 framing、请求编号或 pending map。Rust transport 也不负责 Electron 产品装配。

## 目标位置

按职责选择位置，不要因为“连接 Rust”就把所有文件放进 `electron-main`，也不要用 `contrib` 承接共享连接。

| 职责 | 所有位置 |
| --- | --- |
| 协议方法、DTO、错误和能力协商 | `zeta-rs/app-server-protocol` |
| 请求分派与领域处理 | `zeta-rs/app-server` 及实际领域 crate |
| 服务端传输 | `zeta-rs/app-server-transport` |
| 产品可启动的 Rust 进程入口 | `zeta-rs/server-host` |
| 生成的 TypeScript DTO | `zeta-ts/generated/app-server/types.ts` |
| 共享 client 连接与状态的内部抽象 | `zeta-ts/src/zeta/platform/app-server/common` |
| Node 运行环境相关的 client 机制 | `zeta-ts/src/zeta/platform/app-server/node` |
| Electron Main 连接生命周期与注册 | `zeta-ts/src/zeta/platform/app-server/electron-main` |
| 产品入口的依赖组合 | `zeta-ts/src/zeta/code/electron-main/app.ts` |

以下是职责落位示例，不要求机械创建所有占位文件：

```text
zeta-rs/
├── app-server-protocol/
│   └── src/protocol/<domain>.rs
├── app-server/
│   └── src/request_processors/<domain>.rs
├── app-server-transport/
└── server-host/

zeta-ts/
├── generated/app-server/types.ts
└── src/zeta/
    ├── platform/
    │   ├── app-server/
    │   │   ├── common/appServerConnection.ts
    │   │   ├── node/childProcessJsonlTransport.ts
    │   │   └── electron-main/
    │   │       ├── appServerSupervisor.ts
    │   │       └── appServerConnection.ts
    │   └── <domain>/
    │       ├── common/<domain>.ts
    │       ├── electron-browser/<domain>ChannelClient.ts
    │       └── electron-main/<domain>Channel.ts
    ├── workbench/services/<service>/...
    ├── workbench/contrib/<feature>/...
    └── code/electron-main/app.ts
```

领域文件不需要在 `platform`、`workbench/services`、`workbench/contrib` 三选一。一个领域可以按职责跨层；具体判断读取 [领域适配](domain-adapters.md)。

`node` 只表示代码依赖 Node 运行环境。若原文件只执行已经迁入 Rust 的业务算法，该实现应退出；若它负责 client 侧子进程、stdio 或 Buffer 机制，职责仍然成立。
