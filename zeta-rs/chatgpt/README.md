# `zeta-chatgpt`

`zeta-chatgpt` 负责原生 ChatGPT 订阅 OAuth、profile SecretStore 持久化、refresh token 轮换和请求时 authenticated target 投影。

它为 `openai-chatgpt` 实现 `zeta-login::InteractiveLoginDriver`，并向 `zeta-model-provider` 提供 `ChatGptOAuth::api_target()`。它不拥有 Session、Thread、Turn、工具、批准或 Agent Loop；这些仍属于 Zeta Core 和 App Server。

credential envelope 保存于 `provider/openai-chatgpt/current/oauth`。原始 token 不进入 login snapshot、App Server RPC、Desktop IPC 或 config。

ChatGPT subscription credential 与 OpenAI Platform API key 是独立的凭据和计费路径，两者不会互相降级。

完整架构与兼容性边界见 [`docs/chatgpt-subscription.md`](../../docs/chatgpt-subscription.md)。
