# `zeta-app-server`

`zeta-app-server` 组合一个环境中的服务并实现 App Server 协议，具体职责只有三项：

1. 在每条已建立的连接上执行类型化协议分发，并编排 Thread、Turn、WorkRun、Project 与通知生命周期；连接建立、鉴权和消息队列由 `zeta-app-server-transport` 负责。
2. 在文件、搜索、Git、Terminal、语言服务和目录贡献入口检查对应 Permission，并只把有效 `Authorization` 交给执行服务。
3. 组合 profile 级配置与产品服务；Project 只保存弱关联，WorkRun 验证与集成只接受 host 重建的证据，目录配置、Instructions、Hooks、Skills、MCP 和 Plugins 只有在获得对应 Permission 与 Grant 后才能生效。

环境和目录授权语义见 [`docs/environment-access.md`](../../docs/environment-access.md)，wire contract 见
[`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)。

```text
just test zeta-app-server
```
