# zeta-tui

Zeta 交互模式的 presentation shell。它负责：

- CLI 启动与配置加载；
- embedded、local daemon、remote App Server 的连接选择；
- App Server typed JSON-RPC 调用；
- Session、Thread、sub-agent 和 side conversation 的只读客户端投影；
- 键盘输入、输入框、弹窗与审批决定采集；
- 流式消息、工具调用和 Markdown 呈现；
- terminal raw mode、alternate screen、scrollback 和 resize/reflow。

TUI 不拥有 Session、Thread、Turn 或 ThreadItem 的权威状态机。Canonical 模型和
sequence/cursor 语义见 [`docs/protocol.md`](../../docs/protocol.md)。

durable sequence 或 stream cursor 出现空洞、未知状态或冲突时，TUI 通过 App Server
`session/read` / `thread/read` 重新同步；不能从日志文本或不完整 delta 推断完成状态。
connection session、terminal session 与产品 `Session` 是不同生命周期，代码和文档不得
使用一个无修饰的 `Session` 类型混用它们。
