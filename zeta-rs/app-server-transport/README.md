# `zeta-app-server-transport`

`zeta-app-server-transport` 隔离 App Server 的连接能力，具体职责只有三项：

1. 建立仅限回环地址、能力令牌鉴权的 WebSocket 连接，并为每条连接提供独立的读写通道和确定的关闭语义。
2. 对单条消息大小和双向队列施加固定上限；慢连接只阻塞或关闭自身，不影响同一进程中的其他连接。
3. 提供 daemon 使用的本地套接字接入与带期限的数据流；不解析 JSON-RPC，不执行 `initialize`，也不拥有领域状态。
