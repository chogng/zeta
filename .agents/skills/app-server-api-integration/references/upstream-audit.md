# 上游调用链调查

替换或评审任何 TypeScript / Node 后端能力前，完整读取本文件。目标不是寻找相似文件名，而是证明 VS Code 前端真正依赖的行为，以及 Rust 必须接管的边界。

## 调查产物

先建立一张事实表，再开始设计：

| 项目 | 必须记录 |
| --- | --- |
| 用户行为 | 哪个可观察行为触发调用，成功和失败如何呈现 |
| 保留的前端 | browser、editor、workbench 中继续存在的调用方和领域契约 |
| 被替换的后端 | TypeScript / Node 中执行系统访问、长任务、状态管理或业务规则的实现 |
| Rust 缺口 | 当前 Rust 协议和领域能力尚未表达的行为 |
| 适配边界 | 哪一层把 VS Code 领域值转换成生成 DTO，并把结果还原回来 |
| 验证证据 | 对应测试、注册点、调用方搜索和删除旧入口后的检查 |

## 追踪 VS Code

沿真实 imports、服务注册和 channel 注册追踪下列完整链路：

```text
用户行为
  → browser / editor / workbench 调用方
  → common 领域接口或 channel client
  → Electron Main 的领域 IServerChannel / Node 实现
  → 进程注册和生命周期 owner
```

对每一段记录方法签名、事件、取消、错误、资源释放、对象标识和顺序要求。不要只记录 request/response；watch、终端输出、搜索进度和进程退出通常决定协议形态。

优先从这些入口开始，并沿当前 checkout 继续追踪：

| 需要确认 | 调查入口 |
| --- | --- |
| VS Code channel、调用、事件和传输级取消 | `../vscode/src/vs/base/parts/ipc/common/ipc.ts` |
| 远端连接如何用一个 client 承载多个 channel | `../vscode/src/vs/platform/remote/common/remoteAgentConnection.ts` |
| 文件系统客户端与 Electron 服务端 | `../vscode/src/vs/platform/files/common/diskFileSystemProviderClient.ts`、`../vscode/src/vs/platform/files/electron-main/diskFileSystemProviderServer.ts` |
| workbench 文件服务如何组合 platform provider | `../vscode/src/vs/workbench/services/files/electron-browser/diskFileSystemProvider.ts` |
| 终端契约、channel client 和 workbench 接入 | `../vscode/src/vs/platform/terminal/common/terminal.ts`、`../vscode/src/vs/workbench/contrib/terminal/common/remote/remoteTerminalChannel.ts`、`../vscode/src/vs/workbench/contrib/terminal/browser/remoteTerminalBackend.ts` |

这些路径只是调查入口，不是复制模板。以实际符号、调用方、注册点和测试为准。

## 追踪 Codex

只调查 Rust app-server 的后端机制：

| 需要确认 | 调查入口 |
| --- | --- |
| 协议定义、方法注册和 TypeScript 生成 | `../codex/codex-rs/app-server-protocol` |
| 请求处理、通知和服务端发起请求 | `../codex/codex-rs/app-server` |
| JSONL 连接、帧和传输限制 | `../codex/codex-rs/app-server-transport` |

检查协议如何关联请求、分派方法、发送通知、处理服务端请求、关闭连接和生成类型。不要把 Codex 当前拥有的领域方法当成 Zeta 的能力清单，也不要把 thread、turn、item 等 Codex 前端概念带入无关的 VS Code 领域。

当 VS Code 契约要求的能力在 Codex 协议中不存在时，结论应是“Rust 协议需要新增该能力”，而不是“前端改成 Codex 已有模型”。

## 划定替换线

逐项给文件或符号标记以下一种结论：

- **保留**：前端领域接口、调用方、UI 状态和 VS Code 可观察语义。
- **改接**：channel client、Electron Main adapter、进程装配等边界代码，职责仍存在但实现改为调用 Rust。
- **替换**：已由 Rust 接管的 TypeScript / Node 业务算法、系统访问、后端状态和旧进程入口。
- **新增**：VS Code 契约需要但 Rust 尚未提供的协议与领域能力。

只有当每个生产调用方都能落入这四类之一、且替换项不再拥有必要调用方时，调查才完成。删除文件仍需用户确认准确路径。
