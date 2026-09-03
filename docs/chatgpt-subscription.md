# ChatGPT 订阅接入

> 状态：部分具备。登录、刷新、脱敏账户摘要、固定 Responses 目标与本地 Agent Loop 已接通；兼容探测、丰富响应项、额度/限流状态和完整故障矩阵仍待收口。
> 认证实现：`zeta-rs/chatgpt/`
> 模型执行：`zeta-rs/model-provider/`
> 登录控制面：[`login.md`](login.md)
> SecretStore：[`secrets.md`](secrets.md)
> 官方依据：[ChatGPT 登录](https://learn.chatgpt.com/docs/auth)

## 快速理解

Zeta 直接接入用户的 ChatGPT 订阅，不启动外部 Agent runtime，也不把 Thread、Turn、工具、批准或恢复委托给另一个进程。

`zeta-chatgpt` 只负责本机 OAuth、token refresh、SecretStore envelope 和请求级认证 header；`zeta-model-provider` 使用该 authority 调用 ChatGPT subscription Responses target；完整 Agent Loop 继续由 Zeta Core 与 App Server 持有。

ChatGPT 订阅与 OpenAI Platform API key 是两套不同的 credential 和计费边界，不能互相回退、转换或复用。

| 场景 | 所有者 | 行为 |
| --- | --- | --- |
| 登录与 refresh | `zeta-chatgpt` | 本机 OAuth 与 SecretStore envelope |
| 单次模型请求 | `zeta-model-provider` | 使用 fresh subscription target 调用 Responses |
| Agent Loop | Zeta Core 与 App Server | 持有 Thread、Turn、工具、批准、恢复与 steering |
| Platform API key | direct-provider credential layer | 独立计费和 target，不与订阅互换 |

## 数据流

```text
Settings / account RPC
        │
        ▼
zeta-login ──▶ zeta-chatgpt ──▶ OpenAI OAuth
                    │
                    └──▶ profile SecretStore

Zeta TurnExecutor
        │
        ▼
zeta-model-provider ──▶ zeta-chatgpt fresh target
        │
        └──▶ ChatGPT subscription Responses service
```

静态模型目录使用 `access = subscription` 表达用户接入方式，并使用 `runtime = chatgpt_subscription` 选择认证 target。模型身份仍是 `openai/<model>`；登录账户 identity 是 `openai-chatgpt`，不会进入 `ModelRef`。

## 本机 OAuth

当前产品使用 OpenAI device authorization：Zeta 请求一次性 code，Desktop 打开验证页并复制 code，本机 worker 在可取消的 15 分钟窗口内轮询并交换 token。成功后，access token、refresh token、ID token、expiry、脱敏账户 metadata 和 credential revision 作为一个 opaque envelope 保存到 `provider/openai-chatgpt/current/oauth`。

OAuth access token 接近过期时，`zeta-chatgpt` 在单一 refresh 临界区内刷新并原子替换 envelope。模型请求只获得一次调用所需的 `ResolvedApiTarget`；token 不进入配置、RPC、日志或前端状态。

## Agent Loop 所有权

ChatGPT subscription model 与 API-key model、Kimi subscription model 一样进入本地 `TurnExecutor`。Core 继续拥有 durable history、上下文压缩、工具循环、批准、steering、恢复、预算和多 Agent 协调。

这条边界是架构约束：认证 adapter 不能创建远端 Thread/Turn，也不能实现 Core `TurnExecutionBackend`。

## 兼容性边界

OpenAI 官方文档明确区分“Sign in with ChatGPT”订阅访问与 API key 的 usage-based 访问。当前 direct Responses target、OAuth client/endpoint 和 header contract 依据 OpenAI 开源实现保持兼容；它们不是一份独立发布的通用公共 API 合约，因此升级时必须用登录、refresh、streaming response 和错误分类测试验证，不能静默改写为 Platform API key。

## 当前状态与待完成项

阶段性的 Agent Loop 总计划已经退场，ChatGPT 订阅路径的状态与完成门由本文长期维护。

| 能力 | 状态 | 完成门 |
| --- | --- | --- |
| OAuth 与 Responses 目标 | 进行中 | 对真实服务做版本漂移探测；登录或响应合约不兼容时安全失败并给出可行动错误，绝不改用 Platform API key。 |
| 丰富响应项 | 尚未完成 | 把订阅 Responses 支持的响应项映射为统一的持久化 Item 与通知；重连后只从统一状态重建，Desktop 不依赖供应商 DTO。 |
| 图片输入 | 已实现 | 继续通过工作区附件授权、MIME、字节与像素边界进入受控的模型输入。 |
| 敏感交互输入 | 进行中 | 按 [`secrets.md`](secrets.md#8-交互式敏感输入) 的一次性交付边界响应 `isSecret` 请求，不进入普通 transcript、Thread Item、错误、Debug 或观测数据。 |
| 账户摘要与限流状态 | 部分具备 | 本机 OAuth 已把脱敏的账户、组织、方案、状态和凭据版本投影到独立账户状态；仍需增加额度与限流观察。过期观察必须变为未知，不能门禁静态模型目录，真实失败仍归属准确 Turn。 |
| 登录与流式故障矩阵 | 尚未完成 | 覆盖 device poll、refresh rotation、401、429、流截断、取消和恢复；token 不泄露，不确定的模型调用结果不重放，所有等待都有终态。 |

升级或发布这条路径前，至少运行登录、刷新、流式、重连、错误分类和脱敏测试。任何线上字段无法映射为统一 Item 时必须明确失败或标记未支持，不能静默丢弃。
