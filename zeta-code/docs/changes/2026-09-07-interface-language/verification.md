# 界面语言设置：验收

## 2026-09-07 · 工作区候选

结论：满足 v1 的四条验收要求。Config 根页摘要已移除，四种界面语言可切换并持久化，保存后页面立即刷新；本轮明确不翻译后端内容、对话正文和 Config 子页面。

- 输入：[intent.md](intent.md) v1 与 [spec.md](spec.md) v1。
- 候选：`7ee335295` 加未提交源码与测试差异 SHA-256 `ae0abbd6662498670d8daa2859898aec4006f0db875e14a60ee8273f4ad1445f`。
- 环境与时间：macOS arm64，真实 PTY 使用仓库 `zeta-cli` 场景宿主，2026-09-07。
- 审查：自查配置所有权、显式状态流、键盘路径、无颜色表达、持久化边界和最终差异。

| AC / INV | 实现情况 | 实际证据或命令 | 验证结果 |
| --- | --- | --- | --- |
| AC-1 | Config 根页只保留四个可操作设置，Providers 与 Language servers 页签不变 | `just test zeta-tui --lib config::`：35 项通过 | 通过 |
| AC-2、INV-2 | English、日本語、中文、Français 双向循环；Enter、Space、左右键和鼠标激活共用动作路径，当前值以文字显示 | `just test zeta-tui --lib language`：14 项通过 | 通过 |
| AC-3、INV-1 | `[tui].language` 使用四个有界代码，拒绝未知值并保留其他字段 | 同上；App Server 持久化测试包含 `zh-CN` | 通过 |
| AC-4 | `InterfaceLanguage` 与 `InterfaceText` 统一解析 Config 根页文字，保存完成事件重建打开页面 | 同上；中文页面状态测试通过 | 通过 |
| AC-2、AC-3、AC-4 | 从真实 `/config` 页面选择日语，立即显示 `拡張 TUI`，退出后配置含 `language = "ja"` | `just test zeta-cli --test tui_real_scenarios actual_tui_switches_interface_language_and_persists_it -- --exact`：1 项通过 | 通过 |

相关构建与检查：

- `just check zeta-tui` 通过；输出只有仓库已有的未使用辅助方法警告，本次未新增警告。
- `git diff --check` 通过；现行文档引用目标存在。
- 首次配置测试在链接阶段因磁盘空间耗尽失败；清理 29GB 可重建增量编译缓存后，同一测试通过。
- 真实 PTY 首次运行因等待已退场的 Welcome 文案失败；改用当前稳定的版本标题作为启动条件后，同一行为场景通过。

## 2026-09-07 · NLS 命名整理候选

结论：按后续命名决定将统一文案接口收敛为 `nls`，持久化格式和语言切换行为不变。当前工作区同时包含另一项 Memory diagnostics 设置改动；本次保留其逻辑，并将它的 Config 文案接入同一 NLS 映射。

- 输入：用户要求采用 `nls` 命名。
- 候选：`7ee335295` 加当前未提交源码与测试差异 SHA-256 `708468c0b06a5b44c52d57b2cd00681a966e60daf5bca203591d4bb818d91837`。
- 审查：自查旧标识清理、消息作用域、最新 Config 条目顺序和文档链接。

| 检查 | 实际证据或命令 | 结果 |
| --- | --- | --- |
| NLS 接口 | `just test zeta-tui --lib nls`：3 项通过 | `nls::Language`、`nls::Message`、`nls::text` 与中文 Config 页面通过 |
| 语言行为 | `just test zeta-tui --lib language`：14 项通过 | 切换、解码、保存、刷新和语言服务器回归通过 |
| 真实 PTY | `just test zeta-cli --test tui_real_scenarios actual_tui_switches_language_and_persists_it -- --exact`：1 项通过 | 日语即时刷新和 `language = "ja"` 持久化通过 |
| 生产构建 | `just check zeta-tui` | 通过；只有仓库已有的未使用辅助方法警告 |
| 格式与差异 | `rustfmt --edition 2024 --check ...`、`git diff --check`、生产代码与现行文档旧标识搜索 | 通过；历史候选保留当时名称，当前实现没有旧标识残留 |

## 文档合入

| 现行规格或设计 | 合入的要求 | 功能总览 | 历史入口 |
| --- | --- | --- | --- |
| `docs/spec/commands.md`、`docs/design/tui.md`、`tui/README.md` | AC-1 至 AC-4、INV-1、INV-2 | `docs/capabilities.md` 已更新 | `docs/changes/README.md` 已登记 |

完成范围与日期：Config 根页精简、四语言设置、统一文案接口、profile 持久化与即时刷新于 2026-09-07 完成；其他页面与内容翻译不在 v1 范围内。
