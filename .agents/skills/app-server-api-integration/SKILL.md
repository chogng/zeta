---
name: app-server-api-integration
description: Replace VS Code TypeScript or Node backend capabilities with a Codex-style Rust app-server while preserving the VS Code TypeScript frontend contracts. Use when designing, implementing, or reviewing the replacement API, its requests, notifications, streams, cancellation, server-initiated requests, or TypeScript adapter placement; do not use for same-process TypeScript services or Rust-only domain logic.
---

# VS Code 前端改接 Rust App-server

始终从同一个终局模型出发：保留完整的 VS Code TypeScript 前端及其 browser、editor、workbench 领域契约，把原来由 TypeScript 或 Node 执行的后端能力替换为 Rust app-server。Codex 只作为 Rust 协议、传输、请求处理、双向消息和类型生成的架构参考，不提供前端领域模型。

VS Code 前端需要什么能力，由真实调用链和已有契约决定。Codex 当前协议缺少文件、终端、搜索或其他能力时，扩展 Rust 协议；不要改变 VS Code 前端去迁就 Codex 已有 API。

## 开始前

- 完整读取仓库说明、匹配目标文件的 scoped instructions，以及目标子树最近的 `AGENTS.md`。
- 修改、增加、移动或评审 VS Code 对应的 TypeScript 文件时，同时使用 `vscode-api-alignment`，并读取它为目标层指定的 reference。
- 将 `../vscode` 和 `../codex` 作为只读证据。先追踪当前 checkout 的符号、注册点、调用方和测试，再决定接口和位置。
- 保存 `git status --short` 和目标 diff，保护用户已有改动。

## 按任务读取 references

不要一次读取全部 reference。先判断当前任务涉及哪些决策，再完整读取对应文件。

| 当前任务 | 必须读取 |
| --- | --- |
| 替换或评审任一 TypeScript / Node 后端能力 | [上游调用链调查](references/upstream-audit.md) |
| 设计进程、连接、IPC、握手、重连，或决定连接文件放置 | [连接与文件放置](references/connection-and-placement.md) |
| 决定文件、终端、搜索等领域契约和适配器放在哪里 | [领域适配](references/domain-adapters.md) |
| 设计请求、通知、订阅、流、取消、服务端请求、错误或并发 | [协议语义](references/protocol-semantics.md) |

实现完整领域接入时通常需要读取四份；只做局部评审时读取与评审范围直接相关的文件。

## 始终成立的约束

- **VS Code 前端契约是接口基准**：保留调用方可见的接口、事件、取消、流、生命周期和错误语义。不得从 Codex DTO 反推前端服务。
- **两段连接不能混成一层**：renderer 与 Electron Main 使用 VS Code 领域 IPC；Electron Main 独占与 Rust app-server 的线上连接。renderer 不获得通用 JSON-RPC 连接。
- **一条 Rust 连接承载多个领域**：同一前端 host 到同一后端目标的一次会话共享连接、握手、请求关联和连接状态；文件、终端、搜索等仍暴露各自的领域 channel 与服务。
- **Rust 协议是线上类型的唯一来源**：请求、响应、通知、错误和服务端发起的请求先在 `zeta-rs/app-server-protocol` 定义，再生成 `zeta-ts/generated/app-server/types.ts`。禁止手写第二套等价 DTO 或方法名。
- **生成类型只停留在边界**：只有连接实现和领域适配器使用生成 DTO；browser、editor、workbench 产品代码依赖 VS Code 领域契约。
- **每项职责只有一个 owner**：一个领域可以按职责跨越 `platform`、`workbench/services` 和 `workbench/contrib`，但同一状态、策略或后端能力不得存在两个实现。
- **TypeScript 适配器保持机械**：只做 DTO 转换、关联标识、错误映射和生命周期衔接。策略、权限、持久化、排序、业务校验和后端状态属于 Rust 领域能力。
- **替换必须收敛**：迁入 Rust 的业务逻辑不保留 TypeScript / Node 并行实现、失败切回旧后端的路径或临时桥。

## 工作流程

1. 按 [上游调用链调查](references/upstream-audit.md) 追踪一条完整链路，明确标记保留的 VS Code 前端和由 Rust 替换的 TypeScript / Node 后端。
2. 写出调用方契约、Rust 能力缺口、消息形态、生命周期、取消、错误和并发要求；按任务路由读取其余 references。
3. 先在 Rust 定义完整协议和领域处理入口，再生成 TypeScript 类型。不得先在 TypeScript 发明线上接口。
4. 在 VS Code 对应 owner 中实现薄适配器；Electron Main 负责 Rust 连接和领域 channel 注册，总装配文件只组合依赖。
5. 同批迁移生产调用方并让旧后端入口退出。删除文件前仍需用户确认准确路径。
6. 添加协议、适配器和真实行为的最小测试，执行定向检查，并确认生成 DTO、方法名和后端状态没有越过边界。

## 必须停止并报告的情况

- VS Code 前端要求的流、watch、终端持续输出、取消或其他语义无法由当前 Rust 协议完整表达。
- 真实调用链仍无法判断同一职责的唯一 owner。
- 需要新增 VS Code 中没有对应职责的 Zeta TypeScript 公开路径，用户尚未确认该职责。
- 设计要求 UI 或产品代码直接使用生成 DTO。
- 旧 Node 实现仍有必要调用方，而当前任务没有授权改变其行为。

不要用兼容外壳、空实现、运行时方法字符串、`unknown` DTO 或失败后切回旧后端解除阻断。

## 验证底线

- Rust：运行目标协议、schema 和领域能力的最小测试，遵守仓库 `just check`、`just test` 规则。
- TypeScript：运行目标适配器和调用方的最小 typecheck 与行为测试。
- 边界：用 `rg` 检查生成类型没有扩散到适配器之外，线上方法名和 DTO 没有在 TypeScript 中重复定义。
- 收敛：用 `rg` 确认旧入口、旧 Node 业务实现和并行状态 owner 已无生产调用方。
- 最后检查 `git diff` 与 `git status`，分别报告本次改动、用户已有改动、实际测试和未解决限制。
