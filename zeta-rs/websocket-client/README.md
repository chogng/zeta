# `zeta-websocket-client`

## 快速理解

`zeta-websocket-client` 是 provider-neutral 的 WebSocket transport。它复用
`zeta-http-client::OutboundNetworkSnapshot` 的 proxy、TLS/mTLS、connect timeout 与网络目标策略，只负责
upgrade、bounded frame/message、ping/pong/close 和原始 text/binary message。

Provider 的 JSON event、`previous_response_id`、turn state、prewarm、session 复用、HTTP fallback 与
operation retry 不属于本 crate；这些状态由后续 `zeta-api` codec 和 `zeta-model-provider` 的
`ModelClientSession` 拥有。

当前实现提供 `WebSocketConnector`、`WebSocketConnection`、`WebSocketRequest` 与
`WebSocketMessage`。Public API 不暴露 Tungstenite stream、sink、frame 或 error；request URL、header
value 与 proxy credential 不进入 `Debug` 和 transport error。

Provider 是否可以使用本 transport，不能根据 OpenAI-compatible HTTP API 猜测。
`ProviderDefinition.websocket_api_profile` 是 exact wire protocol 的第一道 fail-closed authority；真实
调用还必须由 runtime service target 单独允许。上游支持矩阵见
[`docs/model-provider.md`](../../docs/model-provider.md)。

## 边界

```text
zeta-model-provider ModelClientSession   [尚未接入]
                  │
        zeta-api WebSocket codec         [尚未接入]
                  │ raw owned messages
                  ▼
        zeta-websocket-client            [当前已实现]
        ├─ handshake + connection
        ├─ bounded message/frame
        ├─ direct/proxy TCP route
        └─ WS/WSS + TLS/mTLS
                  │
     zeta-http-client outbound policy
```

本 crate 不维护 Agent loop、conversation、tool execution 或 provider authentication。OAuth/API key
由上层组装为 handshake header；transport 仅转发并保证 debug redaction。

## 测试

```text
cargo test -p zeta-websocket-client
bazel test //zeta-rs/websocket-client:websocket-client-unit-tests
```

单元测试只使用本地 echo server，不访问真实 provider。
