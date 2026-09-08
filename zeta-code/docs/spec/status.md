# 状态、用量与进程资源

- 入口：常态 `StatusLine`、`/status`、`/statusline`；持续诊断另见[内存诊断](memory-diagnostics.md)。
- 本文维护用户可见读数、统计范围及何时更新；后台线程、版本和队列见[采样设计](../design/process-resources.md)。

## 状态行与 Status 面板

| 区域 | 内容或操作 |
| --- | --- |
| StatusLine | 权限模式、模型、Git、Plan、Subagent 摘要，可选本机常驻内存与 CPU；Queue 在输入框上方独立显示 |
| `/statusline` | 选择条目，Enter / Space 开关；Esc 关闭 |
| Thread 页 | 当前模型、上下文窗口、累计模型调用数、输入/输出 token、缓存读取/写入 token、缓存读取占比、推理输出 token、累计参考费用、Session/Thread 身份 |
| Processes 页 | 只读显示诊断状态、本机常驻内存、观察峰值、1 分钟与 5 分钟变化、CPU，以及 TUI 与本地 App Server 进程树明细；不提供开关 |
| 切页 | Tab / Shift+Tab 或左右循环切页；复用统一页签样式，不加方括号；每页保留滚动位置 |
| 阅读 | ↑↓ / k/j、PageUp/PageDown、Home/End；操作提示为 `Tab to switch · Esc to close`，不重复列出基础导航 |
| 高度 | 按两页最大完整内容申请并受当前空间限制，详见[布局](layout.md#status-面板) |

## 用量口径

| 指标 | 含义与缺失处理 |
| --- | --- |
| 缓存读取占比 | 缓存读取 token / 总输入 token，不是请求次数命中率 |
| 部分报告缺失 | 已知非零 token 用 `>=` 标为下界，已知费用用 `≥` 标为下界；没有可信值时显示 `unknown` |
| Session 与 Thread | Session 用量可合计全部 Thread；上下文窗口属于单条 Thread，Status 的 Thread 页只显示当前对话 |
| Agent 资源 | 不推测每个 Agent 分摊的 CPU 和内存 |

## 本机进程的范围

| 对象 | 计入方式 |
| --- | --- |
| 当前 TUI | 始终计入本机合计 |
| CLI 明确登记 PID 的本地 App Server | 包含该进程及全部后代；先显示进程树合计，再显示主进程和子进程层级 |
| 子进程 | 使用操作系统名称与 PID，不从命令参数猜测 LSP 或工具角色 |
| 进程内 App Server | 已包含在 TUI 读数内，不重复相加 |
| 远程 App Server | 明确标为排除，不混入本机合计 |
| 不在本地 App Server 进程树内的工具 | 不计入任何一项 |

## 读数与观测周期

| 条件 | 结果 |
| --- | --- |
| 内存 | 使用常驻内存口径 |
| CPU | 使用进程占本机逻辑 CPU 总容量的比例，计算需要两个时间点 |
| 新观测周期 | 立即读取内存；CPU 下一次读数前显示 `collecting` |
| 读取失败、目标退出、系统不支持 | 对应项显示 `unavailable`；应计入的本地进程不可读时，本机合计也不可用 |
| 观察峰值与趋势 | 只描述有连续样本覆盖的当前周期，不表示进程全生命周期峰值 |
| 暂停后恢复 | 不跨未采样时段计算 1 分钟或 5 分钟变化；重新出现的指标不能把旧值当作新读数 |

## 何时采样

| 可见需求 | 周期 | 指标 |
| --- | --- | --- |
| 状态行实际排入内存或 CPU | 2 秒 | 只采实际显示的指标 |
| Processes 为当前页签且有正文空间 | 1 秒 | 完整资源信息 |
| Thread 页 | 不单独采样 | 该页本身不提出进程采样需求 |
| 没有可见消费者 | 停止 | 不读进程数据，不产生采样事件 |

- “实际显示”由配置、宽度和行数共同决定，仅启用配置不足以触发采样。
- 两处同时可见时，使用最完整的指标集合和最短周期。
- 在状态行与 Processes 之间切换，不中断仍在采样的指标；内存没有消费者时结束周期并清除峰值和趋势。
- 观测失败不影响输入、任务、启动或退出；内存增长只是排查线索，不能单独证明泄漏。

## 相关记录

- [功能完整性核对](../changes/tui-completeness/README.md)：AC-27，按需读数。
- [内存诊断配置开关](../changes/2026-09-08-memory-diagnostics-config/README.md)：Config 启停与 Status 只读边界。
- [共享内存诊断验收](../../../zeta-rs/docs/changes/memory-diagnostics/verification.md)：后端持续采集能力。
