# `zeta-thread-store`

- 定义 `ThreadStore` 的完整读取、原子追加、sequence 冲突和 batch 校验；持久记录格式由 `zeta-history` 拥有。
- Thread 事件流是对话、Turn、Item、交互、分支关系和 `session_id` 的唯一持久事实源；没有独立 Session store。
- 后端必须 complete-or-none 提交并保留精确顺序；恢复与 reducer 属于 `zeta-core`，SQLite 实现属于 `zeta-state`。
