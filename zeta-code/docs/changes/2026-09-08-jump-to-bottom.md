# 恢复正文回到底部点击入口

恢复 `Jump to bottom (click) ↓`，Ctrl+End 保持可用；不把两种操作说明拼成长文案，也不改变 TopTip 或默认 StatusLine。

长期要求归属[正文](../spec/transcript.md)、[样式](../spec/styles.md)与[终端](../spec/terminal.md)，架构归属 [TUI](../design/tui.md)。本次是已有控件的恢复修复。

## 要求与实现

| 验收项 | 行为 | 实现 |
| --- | --- | --- |
| AC-1 | 离开最新位置后，正文视口末行显示短按钮；文字和箭头均可点，点击恢复跟随并隐藏按钮 | 正文视图共享绘制与命中区域；鼠标和键盘调用同一回到最新动作 |
| AC-2 | 增强鼠标关闭后不响应点击，提示改为 `Ctrl+End to jump to bottom ↓` | 根据当前鼠标能力选择文案与命中行为 |
| AC-3 | 预览仅滚动自身，详情浮层阻止操作背景；保留输入草稿、TopTip 与默认底栏 | 沿用当前输入上下文，补充隔离与底栏断言 |
| AC-4 | 窄窗口裁剪后的可见按钮仍可点击 | 16 列与 50 列命中测试覆盖文字区域两端 |

## 验证

2026-09-08，Windows PowerShell；基线 HEAD `b00ee076e`，候选包含工作区原有全屏终端改动及本次修复，未提交。自查，未进行独立审查。

- `just check zeta-tui`：通过。
- `just test zeta-tui`：700 项通过，2 项仅因预期文案变化失败，1 项要求 PTY 的测试忽略；检查并接受对应快照后分别重跑下列两项。
- `just test zeta-tui session_manager_preview_reads_conversation_and_restores_focus_without_editing`：通过。
- `just test zeta-tui welcome_header_remains_at_the_start_of_scrollable_history`：通过；累计 702 项通过，1 项忽略，无剩余失败。
- `just test zeta-cli --test tui_real_scenarios actual_tui_scrolls_the_transcript_with_the_mouse_wheel -- --nocapture`：真实 Windows PTY 通过，验证滚轮进入历史、点击返回 `line 20` 以及控件隐藏。
- 三份字符快照只改变回到底部文案和居中空格。环境未安装 cargo-insta，逐份检查 `.snap.new` 后复制到各自对应基线，未批量接受其他快照。

本次没有重跑 Unix 终端。AC-1 至 AC-4 的状态、事件与 Windows 终端证据见上述测试。

## PR #20 合并主分支后的复验

2026-09-08，将 `origin/main` 的 `7692f5860` 合入 PR 分支 `7c5e8d0d0`。保留全屏正文布局、回到底部控件、Issue 管理器及双方 PTY 场景；补回 Issue 上下文需要的当前线程 ID 读取，并阻止 Issue 页面上的鼠标操作改变背景正文。

- `just check zeta-tui`：通过。
- `just test zeta-tui -- --quiet`：728 项通过，1 项忽略；包含 Issue 页面高度与背景鼠标隔离回归。
- Windows PTY 首轮因配套 daemon 二进制仍使用旧协议而在 TUI 启动前失败。`just test-tui actual_tui_issue_config_switch_gates_its_tab -- --nocapture` 构建匹配的 daemon 后通过。
- `just test zeta-cli --test tui_real_scenarios actual_tui_scrolls_the_transcript_with_the_mouse_wheel -- --nocapture`：通过。
- 固定输入区快照首轮受到本机剪贴板图片提示影响；测试改为等待剪贴板与权限临时提示消失后取稳定画面。检查并更新 `issue13/fullscreen_conversation`，正文与输入区位置不变；`just test zeta-cli --test tui_real_scenarios actual_tui_input_keeps_hint_bar_without_blank_line_growth -- --nocapture` 在两种尺寸通过，无待接受快照。
- 本次仅自查；Unix 专属 Issue 提交与 PR 场景未在 Windows 执行。
