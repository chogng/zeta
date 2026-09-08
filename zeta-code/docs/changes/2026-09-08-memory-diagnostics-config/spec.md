# 行为要求

| 编号 | 要求 |
| --- | --- |
| AC-1 | `/config` 的 Config 页提供 `Memory diagnostics` 开关，默认关闭；修改使用现有配置 revision，保存到 profile 的 `[tui].memoryDiagnostics`。 |
| AC-2 | 开启后立即开始诊断，重启时按配置重新开始；关闭后停止当前诊断。配置不保存会话身份、样本或报告。 |
| AC-3 | 单段达到后端预算后，在开关仍开启时建立新段；启动失败显示失败且不循环重试，关闭后再次开启才重试。退出或断连释放当前段。 |
| AC-4 | `/status → Processes` 只读显示 `Disabled`、`Starting`、`Recording`、`Stopping` 或 `Failed`，没有启停动作。 |
| AC-5 | TUI 不再注册或执行 `/memory`；诊断开关不调用模型、不改变当前 Turn。 |

INV-1：Status 不能修改诊断配置；Config 不能持有诊断运行状态。
