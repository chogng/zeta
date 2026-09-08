# 验收

2026-09-08；要求版本 [v1](spec.md)，来源与范围见 [意图](intent.md)。本次切页行为已实现并通过下列验收；额外执行的历史导航场景仍有启动文案失效，见限制。

## 候选身份

- Git 基线：`7692f58601433c272ee779edbb015d4adb776067`；开始时工作区无修改，当前包含本次未提交代码和测试差异。
- 代码及测试差异 SHA-256：`a9614f3cfcebc437a80509991b6fb9895d36efa25148f868fdf194d0a2e445ff`。计算输入为 `git diff --binary HEAD -- zeta-code/tui zeta-code/cli/tests/tui_real_scenarios.rs` 的标准输出字节；文档及本证据文件不参与。
- 环境：Windows、PowerShell、Rust 1.98.0、ConPTY；保持默认 Cargo 增量设置。按组件、事件与终端字符自查，未进行独立审查或真实账号认证。

## 要求与证据

| 要求 | 实现及验证结果 | 证据 |
| --- | --- | --- |
| AC-1 | 通过 | 通用列表正文/搜索切页、禁用页跳过、空列表、Repeat/Release；TabList 原有单页/全部禁用测试；Config 正文 Tab 不发出设置修改动作 |
| AC-2 | 通过 | 搜索词保留、供应商未确认 Key 与协议编辑保留；键盘切走后创建回包不抢页；Issue 查询拒绝旧代次 |
| AC-3 | 通过 | Issue 上下键到达两个底部动作；供应商上下字段、末端停止、确认和取消；正文左右仍调整值 |
| AC-4 | 通过 | 隐藏页签不切页或调值；App 快捷键录制捕获 Tab/Shift+Tab；Issue 详情及创建等待不切背景；原有补全、弹窗与输入隔离测试通过 |
| AC-5 | 通过本次范围 | 四项 Windows PTY 场景通过，包括正文与搜索切页、反向切页、保留草稿继续输入、取消后配置不变、窄终端、保存及子面板返回；长期规格、帮助说明及提示同步 |

## 完成的命令

| 命令 | 结果 |
| --- | --- |
| `just test zeta-tui --lib` | 更新提示断言后 743 通过、2 忽略；随后只增补下两项测试并定向执行 |
| `just test zeta-tui --lib shortcut_capture_emits_a_revision_bound_edit` | 1 通过，涵盖 Ctrl+Y、Tab、Shift+Tab |
| `just test zeta-tui --lib tab_does_not_repeat_or_leave_issue_detail_and_pending_creation` | 1 通过 |
| `just check zeta-tui` | 正常构建检查通过 |
| `just test-tui actual_tui_tab_switches_from_content_search_and_unsaved_field` | 配套 daemon、Windows 执行辅助程序和 CLI 构建成功；初次场景断言失败，后续定向重跑见下行 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_tab_switches_from_content_search_and_unsaved_field` | 最终 1 通过；2.47 秒 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_issue_config_switch_gates_its_tab` | 1 通过 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_provider_fields_save_cancel_and_fetch_models` | 1 通过 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_opens_chatgpt_subscription_and_returns_to_openai` | 1 通过 |
| `git diff --check` | 通过 |
| 变更文档本地链接与快照扫描 | 163 处本地链接存在；无待接受快照 |

## 失败记录与限制

- 首轮包测试有 3 项提示文本断言仍期待旧文案；同步后上述包测试通过。一次并行重试遇到 Windows 测试程序占用导致 LNK1104，进程结束后重跑通过。
- 新 PTY 场景初次把启动时的配置规范化写入误认为切页写入；比较基线移到 Config 加载完成后。下一次全部行为断言通过，但清理阶段在供应商面板直接调用退出辅助函数超时；按现有 Esc 逐层返回规则退出面板后，最终场景通过。
- 额外执行 `just test zeta-cli --test tui_real_scenarios actual_tui_navigates_config_tabs_and_temporary_pickers` 失败：等待已删除的 `Tips for getting started` 欢迎页文案，30 秒超时，未执行到本次修改的切页行为。未修改其历史欢迎页或主题快照，不能声称整个 CLI 场景集通过。
- Unix Issue PTY 路径已同步使用方向键到达底部动作，本机无法执行 `cfg(unix)` 场景；Issue 行为由状态和事件回归覆盖。
- CLI 测试支持代码在 Windows 有两个既有未使用方法警告；正常 TUI 构建检查无警告。未运行整个 Rust workspace。
- 没有新增或接受快照基线；本次快照扫描无 `.snap.new`，终端行为以状态、动作、配置文件及字符输出验收。
