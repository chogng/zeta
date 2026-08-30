# `zeta-work-coordination`

1. `WorkCoordinator` 通过版本化命令维护 WorkRun、参与者拓扑、工作契约、工作尝试、关系和冲突。
2. 工作契约绑定一个 Environment 和不可变根检查点集合；Project 关联和 Agent 消息不能扩大授权或改写这些输入。
3. `WorkRunStore` 定义完整记录与命令回执的原子提交；具体数据库、Thread 执行、验证和目标分支更新由调用方实现。

跨组件语义和可靠性完成门见 [`docs/multi-agent-development.md`](../../docs/multi-agent-development.md)。
