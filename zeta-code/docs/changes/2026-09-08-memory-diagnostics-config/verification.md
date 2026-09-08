# 验收记录

候选基于 `7ee335295c` 加当前未提交差异，验证日期为 2026-09-08，环境为 Windows PowerShell。

| 要求 | 结果与证据 |
| --- | --- |
| AC-1 | 通过：设置解析、默认值、完整 `[tui]` 保存、Config 列表项和 revision 绑定测试通过。 |
| AC-2 | 通过：`memory` 功能 owner 只按配置产生开始或停止请求；AppDriver 只负责后台调度，启动时读取配置，关闭后停止，运行状态不写入配置。 |
| AC-3 | 通过：运行状态区分开始、录制、停止和失败；结束的有界段在配置仍开启时重新开始，失败需关闭后再开启才重试。 |
| AC-4 | 通过：Status Processes 字符快照只读显示 `Recording`，面板没有动作或开关。 |
| AC-5 | 通过：`/memory` 已从本地 Slash 枚举、目录、路由和命令处理删除；此前五项 Slash 菜单测试恢复通过。 |
| INV-1 | 通过：Config 只传递布尔设置；Status 只接收展示事件；`memory` 功能 owner 独占诊断句柄和生命周期。 |

| 命令 | 结果 |
| --- | --- |
| `just test zeta-tui --lib config::` | 33 通过 |
| `just test zeta-tui --lib status::panel` | 10 通过 |
| `just test zeta-tui --lib slash_commands` | 4 通过 |
| `just test zeta-tui --lib slash_popup` | 7 通过 |
| `just test zeta-tui --lib bare_slash_renders_the_first_command_window` | 1 通过 |
| `just test zeta-tui --lib` | 699 通过、1 项需要真实 PTY 的既有测试按环境忽略 |
| `just test zeta-app-server-client memory_recording_collects_product_counters_exports_and_stops_for_rust_products` | 1 通过 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_config_enables_and_disables_memory_diagnostics` | Windows 完成构建；测试文件限定 Unix，因此运行 0 项 |
| `just check zeta-tui` | 通过；仅有既有未使用代码警告 |
| `git diff --check` | 通过 |

更新一份 `status_processes_tab` 字符快照，只新增只读的 `Memory diagnostics: Recording` 行；逐行检查后接受。没有待处理 `.snap.new`。真实 Unix PTY 场景已改为从 Config 开启、从 Status 读取、再从 Config 关闭，并断言不调用模型；本机不能执行该场景，因此这项仍待 Unix 环境实测。
