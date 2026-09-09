# `zeta-app-server-protocol`

- 定义 App Server 的 JSON-RPC 请求、结果、通知、错误、方法注册、启动记录、序列化作用域，以及带稳定操作 ID 的领域取消契约；不拥有运行时、连接或存储。
- Session API 只提供按 `session_id` 聚合的 Agent tree；Project 使用独立 revision 和命令回执，不复制 Thread 状态。
- Rust DTO 与方法注册表是唯一协议来源；修改后必须从仓库根运行 `just generate-protocol`，并提交 JSON Schema、三张 TypeScript 方法映射与运行时解码器。
