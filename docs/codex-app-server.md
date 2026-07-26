# `zeta-codex-app-server` 架构

> 计划物理位置：`zeta-rs/codex-app-server/`
> Rust crate：`zeta_codex_app_server`
> 当前状态：Proposed，尚未创建 crate
> 登录控制面：[`login.md`](login.md)
> Provider runtime：[`model-provider.md`](model-provider.md)
> 上游依据：[Codex App Server](https://learn.chatgpt.com/docs/app-server)

## 1. 结论

`zeta-codex-app-server` 是对**外部上游 `codex app-server`** 的本机 adapter。它与
`zeta-app-server` 不是同一个服务：前者是 Zeta 使用的 Codex runtime client，后者是 Zeta 向
Desktop、CLI 和 TUI 暴露的产品 RPC server。

它允许 Zeta 使用用户自己的 ChatGPT/Codex subscription，而不直接调用未承诺稳定的
ChatGPT/Codex backend。上游 Codex 保留 OAuth login、credential persistence/refresh、模型请求
和 Codex backend compatibility 的所有权；Zeta 只适配可检查、版本化的本地 JSON-RPC contract。

## 2. 所有权

`zeta-codex-app-server` 拥有：

- `codex app-server` 子进程启动、健康检查、版本协商和显式 shutdown；
- stdio/Unix socket JSON-RPC transport、request ID、outgoing event dispatch 和 backpressure；
- 上游 `initialize` capability 与 schema/version 的 feature gate；
- `account/login/start`、cancel/logout/account read/rate-limit read 的安全 adapter；
- Codex thread/turn/approval/stream event 到 Zeta provider runtime value 的映射；
- 上游进程故障、版本不兼容、取消和 reauthentication-required 的稳定错误分类；
- 本机 process 和 transport credential 的安全配置。

它不拥有：

- ChatGPT OAuth client、PKCE verifier、callback listener、device-code polling 或 token refresh；
- 读取 `~/.codex/auth.json`、Keychain 或任何上游 token；
- 直接请求 `chatgpt.com/backend-api/codex`；
- Zeta 的 provider registry、catalog cache、Core Thread persistence 或 UI；
- `zeta-http-client` 的通用 Provider HTTP backend。

## 3. 两条 OpenAI 执行路径

```text
OpenAI Platform API key
  zeta-model-provider → zeta-api → zeta-client → zeta-http-client → api.openai.com

ChatGPT/Codex subscription
  zeta-model-provider → injected SubscriptionModelBackend
                      → zeta-codex-app-server → local codex app-server
                      → Codex-managed ChatGPT login → Codex backend
```

两条路径的 credential、endpoint、模型/功能集和错误语义不能互相降级或转换。Platform API key
不能访问 Codex subscription runtime；Codex token 不能作为 `zeta-api` 的 Bearer credential。

## 4. 登录流程

```text
Zeta account/login/start
  → zeta-login
  → zeta-codex-app-server: account/login/start { type: chatgpt }
  ← authorization URL or device-code instructions
  → user completes login in browser
  ← upstream account/login/completed + account/updated
  → zeta-login publishes redacted account state
```

上游 Codex App Server 自己绑定 callback listener、持久化 credential 并正常刷新。Zeta 不应调用
上游标为 internal-only 的 `chatgptAuthTokens` 变体，也不应从其 auth storage 抽取 token。

如果上游 login API、capability 或 schema 不兼容，adapter 必须明确返回 unsupported/version-mismatch，
不能退化为 Zeta 直连私有 backend。

## 5. 与 `model-provider` 的边界

`zeta-model-provider` 定义并消费窄的 `SubscriptionModelBackend` port；Codex adapter 实现该 port。
该 port 表达“以已认证 subscription account 执行模型工作”，而不是暴露 Codex JSON-RPC DTO 或
上游 auth data。

```text
model-provider
  owns: provider/model selection, immutable invocation binding, API-key providers

codex-app-server
  implements: selected Codex subscription model backend

login
  owns: user-visible account/login lifecycle
```

composition root 同时将一个 adapter instance 注入 `zeta-login` 和 model-provider runtime；它们共享
的是受控 runtime，不共享 token 或 provider-global mutable state。

## 6. Versioning and security gates

- 固定并检测支持的 Codex CLI/App Server version range；生成的 upstream schema 是版本特定的。
- account/login 等 upstream method/field 可能受 `experimentalApi` capability gate 约束；adapter
  必须 feature-detect，不能将当前可用形态承诺为永久稳定 API。
- 默认使用 stdio 或 Unix socket；不暴露到公网。远端场景仅允许受保护的 SSH/VPN transport，并单独
  配置 App Server transport authentication。
- child process command line、environment、stderr、RPC trace 和 telemetry 必须过滤 credential、
  authorization URL query、device code 和 workspace-sensitive metadata。
- 认证失败只触发一次明确的 reauthentication-required 状态；不把模型推理 POST 当作可安全重放。
- upstream process crash 使 in-flight operation 的 outcome 为 unknown；由 runtime 的 endpoint-specific
  policy 决定是否恢复，不能由 adapter 盲目重试。

## 7. 落地顺序

1. 读取上游 `initialize`、`account/read` 和 process shutdown，建立 version/transport contract tests。
2. 接入 managed ChatGPT browser/device-code login 与 redacted completion events。
3. 实现一个最小 thread/turn streaming vertical slice，并将其映射为 `SubscriptionModelBackend`。
4. 将 Codex account/plan/rate-limit observation 作为 models-manager 的可选 source evidence；它不是
   catalog authority。
5. 再按实际需要接入 approvals、compact、memory、search 和 realtime；不能先复制所有 upstream
   method。
