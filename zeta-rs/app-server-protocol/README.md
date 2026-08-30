# `zeta-app-server-protocol`

- 定义 App Server 的 JSON-RPC 请求、结果、通知、错误、方法注册和序列化作用域；不拥有运行时、连接或存储。
- Session API 只提供按 `session_id` 聚合的树视图；Thread 写入的 `expected_sequence` 位于具体 request 分支，`session/changed` 仅提示重新读取。
- Rust DTO 是 schema 来源；修改后必须运行 `python3 scripts/write_schema_fixtures.py`，并提交 JSON Schema 与 TypeScript 生成物。
