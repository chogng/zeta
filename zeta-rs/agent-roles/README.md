# `zeta-agent-roles`

`zeta-agent-roles` 是 Agent Role 的唯一 owner：

- 从 `assets/builtins/*.toml` 加载随产品发布的只读 Role，并从目录的 `.zeta/agents/*.md` 加载自定义 Role；两种来源统一生成不可变 catalog、内容摘要和诊断。
- Role 声明职责提示词、模型引用以及 Tool、Skill、Instruction 规则；省略 Tool 或 Skill 列表表示继承调用方，`required_*` 只负责启动前检查。
- 本 crate 不创建 Thread、不调用模型、不授予权限，也不协调多个 Agent；App Server 选择并冻结 Role，Core 只执行冻结后的 Thread。

验证使用 `just test zeta-agent-roles` 和 `just check zeta-agent-roles`。
