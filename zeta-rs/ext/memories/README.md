# Memories Extension

- 通过 `ContextContributor` 为 Turn 首次模型调用提供有界记忆参考材料；Core 负责低信任层级、预算和重试时重新读取。
- 通过 `ReadOnlyToolContributor` 提供 `memories-search` 与 `memories-read`，不提供写入、删除或修改授权的模型工具。
- `MemoryScopeProvider` 由宿主实现，每次根据可信 Session、Thread 身份解析当前可访问范围；工具参数不能指定任务身份或扩大范围。
- 搜索与引用读取都要求对应作用域开启 Memory 读取；默认关闭，撤销、删除和版本冲突在后续读取中生效。
- 依赖 `memories` 的领域契约与 `extension-api` 的扩展契约；不依赖 Core、App Server、产品 UI 或存储实现。SQLite 仅用于集成测试。
- 工具参数和读取边界见 [Memory API](../../../docs/zeta-app-server-api.md#memory)；验证使用 `just test zeta-memories-extension`。
