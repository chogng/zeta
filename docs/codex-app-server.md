# `zeta-codex-app-server` 架构

> 物理位置：`zeta-rs/codex-app-server/`
> Rust crate：`zeta_codex_app_server`
> 当前状态：managed account/login、thread/Turn streaming 与 Core Turn backend adapter 已实现；
> 默认 App Server 已通过显式 `openai-chatgpt` 模型选择接入订阅 Turn
> 登录控制面：[`login.md`](login.md)
> Core 执行边界：[`core.md`](core.md)
> Provider runtime：[`model-provider.md`](model-provider.md)
> 上游依据：[Codex App Server](https://learn.chatgpt.com/docs/app-server)

## 快速理解

Codex App Server 适配器是本机桥接层：它让 Zeta 使用用户自己的 ChatGPT/Codex 订阅，同时把
登录凭据、远端 Agent 循环和私有后端兼容性继续留给上游 Codex 管理。

| 场景 | 正确路径 | 明确禁止 |
| --- | --- | --- |
| 使用 OpenAI Platform API key | model-provider → `zeta-api` → `zeta-client` → HTTP | 转换成 Codex 订阅凭据 |
| 使用 ChatGPT/Codex 订阅 | Core `TurnExecutionBackend` → 本适配器 → 本地 `codex app-server` | 直连私有 ChatGPT 后端 |
| 用户登录 | Zeta 登录控制面委托上游浏览器或设备码流程 | 读取或复制上游 token |
| 上游版本不兼容 | 返回明确的协议或能力错误 | 猜测未声明方法仍然可用 |
| 上游进程崩溃 | 进行中的 Turn 按未知结果失败，不重放 | 盲目重放模型、工具或审批动作 |

## 1. 结论

`zeta-codex-app-server` 是对**外部上游 `codex app-server`** 的本机 adapter。它与
`zeta-app-server` 不是同一个服务：前者是 Zeta 使用的 Codex runtime client，后者是 Zeta 向
Desktop、CLI 和 TUI 暴露的产品 RPC server。

上游 Codex 保留 OAuth login、credential persistence/refresh、模型请求、远端工具循环和 Codex
backend compatibility 的所有权。Zeta 只适配可检查的本地 JSON-RPC contract，并把可持久化的结果
投影回 Core。

### 当前实现

| 能力 | 状态 | 当前边界 |
| --- | --- | --- |
| 懒启动本地 `codex app-server`、stdio JSONL、initialize/initialized | ✅ | 有界 frame、单 writer、exact request ID、进程退出失败传播 |
| `account/read`、browser/device-code start、cancel、logout | ✅ | 只映射脱敏账户与用户可见授权指令 |
| login completion/account updated 主动通知 | ✅ | 上游 login ID 映射为 Zeta-owned ID，兼容早到通知 |
| read-only / workspace-write thread 与 Turn streaming | ✅ | typed thread/start/resume、turn/start/interrupt 与增量事件 |
| command/file approval、structured user input | ✅ | 先建立 durable Core interaction，再 once-only 回答 exact upstream request |
| Core `TurnExecutionBackend` adapter | ✅ | Core 保留 Thread authority；Codex 执行完整远端 Agent loop |
| completed remote thread binding | ✅ | 成功 Turn 后持久化，重建后使用 thread/resume；不持久化或重放 in-flight Turn |
| 默认产品执行 composition | ✅ | `model/list` 投影 account-filtered Codex models；持久化 Turn model 的 exact provider 决定路由 |
| permission approval、diff/image/secret input、rate limit | 尚未完成 | 必须按独立语义切片接入或明确拒绝 |

当前兼容门以 initialize response 与所需 method/shape 为准；还没有声明固定的上游 semver 范围。
method 缺失或响应 shape 不兼容会明确失败，不会改为直连 ChatGPT 私有 backend。

## 2. 所有权

`zeta-codex-app-server` 拥有：

- `codex app-server` 子进程启动、initialize handshake、连接 generation 和显式 shutdown；
- bounded JSONL framing、request ID correlation、response/server-request/event dispatch；
- account/login 的安全 adapter；
- Codex thread/Turn、approval、user-input 与 stream event 的 typed adapter；
- `CodexTurnExecutionBackend` 对 Core item、interaction、cancellation 和 terminal outcome 的映射；
- 上游进程故障、版本不兼容和 unknown-outcome 的 fail-closed 处理。

它不拥有：

- ChatGPT OAuth client、PKCE verifier、callback listener、device-code polling 或 token refresh；
- 读取 `~/.codex/auth.json`、Keychain 或任何上游 token；
- 直接请求 `chatgpt.com/backend-api/codex`；
- Zeta 的模型选择、Core Thread authority、产品 UI 或 HTTP provider runtime；
- 上游未公开能力的兼容猜测。

## 3. 两条 OpenAI 执行路径

```text
OpenAI Platform API key
  zeta-model-provider → zeta-api → zeta-client → zeta-http-client → api.openai.com

ChatGPT/Codex subscription
  Core TurnExecutionBackend → CodexTurnExecutionBackend
                            → CodexTurnDriver
                            → shared CodexAppServerRuntime
                            → local codex app-server
                            → Codex-managed ChatGPT login/backend
```

两条路径的 credential、endpoint、模型/功能集和错误语义不能互相降级或转换。Platform API key
不能访问 Codex subscription runtime；Codex token 不能作为 `zeta-api` 的 Bearer credential。

## 4. 登录与共享 runtime

```text
Zeta account/login/start
  → zeta-login
  → CodexAppServerLoginDriver
  → shared CodexAppServerRuntime
  → upstream account/login/start { type: chatgpt }
```

`CodexAppServerLoginDriver` 与 `CodexTurnDriver` 共享同一份懒启动 runtime 和连接 generation，但不
共享 token。上游负责 callback listener、credential persistence 与 refresh；Zeta 不调用
internal-only `chatgptAuthTokens` 变体，也不读取上游 auth storage。

## 5. Core Turn 执行边界

`TurnExecutionBackend` 由 Core 定义，是“推进一个已持久化 Turn”的 consumer-owned port。默认
`TurnExecutor` 实现 Zeta 自己的 model → tool 循环；`CodexTurnExecutionBackend` 则把完整 Agent loop
委托给 Codex。它不是 raw model invoker，因此不属于 `zeta-model-provider`。

```text
ThreadController accepts/starts Turn
  → selected TurnExecutionBackend.start
  → Codex thread/start or thread/resume
  → Codex turn/start
  ← item deltas / approval request / user input / completed
  → transient Core stream updates
  → durable Core interactions and items
  → terminal TurnCompleted or TurnFailed
```

命令与文件审批、structured user input 必须先成为 durable Core interaction，之后才能回答 exact
upstream JSON-RPC request。重复 response、错误 response kind、旧 connection generation 或未知 request
都被拒绝。含 secret 的 user input 当前直接拒绝，因为 Core 的 durable response 不能存 secret。

默认产品组合把上游 `model/list` 的可用项投影成 `provider = openai-chatgpt` 的 ModelRef；只有用户将
该精确 ModelRef 选入 Session，随后复制到 Turn，App Server 才会选择 Codex backend。登录状态本身
不会切换已有 Session/Thread，也不会改变 direct-provider 路径。

在第一次远程请求前，adapter 先追加 `TurnExecutionAttempted`。因此进程丢失后，已有 attempt 的
in-flight Turn 被视为结果未知并失败，不能重放。成功完成后 adapter 追加 immutable
`TurnExecutionBound`，保存 backend ID、remote thread ID 与 opaque Workspace authority scope。这个
append 发生在 Turn completion 之后，不与 completion 原子提交；若 binding append 失败，Turn 仍保持
completed，但后续不能保证恢复远端 thread continuity。in-flight remote Turn ID 从不持久化或重放。

## 6. 失败与安全语义

- spawn、pipe、framing、timeout、process exit 与 connection close 变成稳定错误；
- active Turn 在连接关闭后失败，不能把“未收到 terminal”解释为“上游未执行”；
- connection generation 阻止旧 server request 被发送到重启后的新进程；
- unknown method 以 JSON-RPC method-not-found 回答，invalid request 以 invalid-params 回答；
- 不支持的 secret input、local image、approval capability 与 schema shape fail closed；
- stderr、credential-bearing payload 与 authorization query 不进入 RPC error 或 telemetry；
- read-only 与 workspace-write 是显式构造选项，workspace 必须是绝对路径。

## 7. 当前限制与下一步

1. 按 canonical product contract 增加 `item/permissions/requestApproval`、diff 更新、image input、
   rate-limit 与 richer completed-item projection。
2. 若要支持 secret user input，先建立不把 secret response 写入 durable Thread event 的专用通道。
3. 固定并测试受支持的 Codex CLI/App Server 版本范围与 capability gate。
