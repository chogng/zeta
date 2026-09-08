# TUI 内存诊断

内存诊断的共享核心归 `zeta-rs`，统一服务 TUI、Rust 图形端和 Electron 桌面端。完整职责、证据规则、生命周期和验收要求见[三端内存诊断设计](../../../zeta-rs/docs/memory-diagnostics.md)。TUI 由 Config 开关控制诊断，Status 只展示状态和资源证据；实际验证范围见[验收记录](../../../zeta-rs/docs/changes/memory-diagnostics/verification.md)。状态行资源采样继续遵守[按需观测设计](../design/process-resources.md)。

## TUI 接入要求

TUI 提供用户明确开始、停止和查看诊断的入口，并报告终端操作阶段及自身运行时证据。诊断会话、进程采样、趋势分析与报告由后端拥有，TUI 不再定义独立算法或保存另一份诊断会话状态。

接入时必须满足：

- 登记 TUI 和明确拥有的后端进程身份；进程内后端只计一次，远程后端与本地 TUI 分开统计；
- 隐藏 Status 不停止 Config 已开启的诊断；
- 用户停止、发起连接断开或 TUI 退出时，终止所属诊断与在途采集；
- 运行时指标不支持、证据不足或后端中断时展示准确状态；
- 展示与导出后端提供的证据，不仅凭常驻内存上涨宣称发生泄漏。

## Config 开关与 Status 展示

- `/config → Config → Memory diagnostics` 是唯一启停入口；设置保存到 profile 的 `[tui].memoryDiagnostics`，默认 `false`。
- 开启后立即建立诊断；重启 TUI 时按配置重新建立。关闭后停止当前诊断。
- 单次后端诊断仍保持最长 30 分钟和有界报告；配置持续开启时，TUI 在上一段结束后建立新段，不把会话 ID、样本或报告写入配置。
- 启动失败显示失败状态和原因，不持续重试；用户关闭后再次开启才重新尝试。
- `/status → Processes` 永远只读显示诊断状态和进程资源，没有独立开关或启停动作。
- TUI 不再注册 `/memory` Slash Command；诊断配置不调用模型，也不改变当前 Turn。

采样约每 5 秒一次。TUI 提供正文单元计数，后端统一分析；目前没有 Rust 分配栈或引用路径采集，因此结果是增长证据，不能自动确认泄漏根因。
