# `zeta-app-server-protocol`

- 定义 App Server 的 JSON-RPC 请求、结果、通知、错误、方法注册和序列化作用域；不拥有运行时、连接或存储。
- Session API 只提供按 `session_id` 聚合的树视图；WorkRun 与 Project 使用独立 revision 和命令回执，`workRun/view/read` 只组合 canonical Agent tree，不建立 Team 事实源。
- Rust DTO 是 schema 来源；修改后必须从仓库根运行 `corepack pnpm generate:protocol`，并提交 JSON Schema 与 TypeScript 生成物。
