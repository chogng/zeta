# 验收记录

环境：2026-09-07，Windows PowerShell。起始 HEAD 为 `dc97603524fdeee0cafc3541f1cf8c755f590dfa`，本次在用户指定工作区修改；同时存在 OpenAI 配置工作的未提交变更，未回退其内容。规格以本目录 [spec.md](spec.md) 为准。

## 行为验收

| 要求 | 实现与证据 | 结果 |
| --- | --- | --- |
| AC-1 | `adding_keeps_existing_items_visible_and_focuses_the_confirmed_directory` 检查输入不筛掉旧项、提交状态与重复 Enter；App 测试检查命令路由 | 通过 |
| AC-2 | `app_add_completion_refreshes_the_open_panel_and_moves_focus` 检查列表刷新、清空输入与 Read files 焦点；`directory_add_feedback_is_visible_below_the_input` 检查字符画面 | 通过 |
| AC-3 | 真实后端测试 `panel_add_uses_server_paths_preserves_permissions_and_reports_missing_directories` 检查相对路径、重复目录、规范路径和权限保留 | 通过 |
| AC-4 | `failed_add_preserves_input_and_can_retry_without_accepting_stale_results` 与空值/多行测试检查错误、保留输入和重试；真实后端检查不存在路径 | 通过 |
| AC-5 | `closing_pending_add_does_not_change_a_reopened_panel` 验证 Esc 与过期完成；会话切换使用既有 close_transient_surfaces 关闭面板 | 通过（App 状态与代码核对） |
| AC-6 | 行内添加/列出/移除的既有集成测试通过；服务端解析与原权限保留由真实后端测试覆盖 | 通过 |

## 执行记录

| 命令 | 结果 |
| --- | --- |
| `just generate-protocol` | 通过；JSON Schema、TypeScript 声明、解码器与前端生成目录已更新 |
| `just test zeta-app-server-protocol --lib` | 38 项通过，包含生成产物一致性 |
| `just test zeta-tui --lib dirs:: -- --test-threads=1` | 9 项行为测试通过；首次字符快照等待基线审阅 |
| `just test zeta-tui --lib directory_add_feedback_is_visible_below_the_input` | 已审阅并接受唯一目录快照，重跑通过 |
| `just test zeta-tui --lib widgets::list_selection::` | 28 项通过 |
| `just test zeta-tui --lib add_dir_adds_lists_and_removes_the_exact_session_directory` | 1 项通过 |
| `just check zeta-tui` | 通过；最后一处绘制返回值调整后的复查也通过 |
| `just test zeta-app-server-client --lib client_manages_session_dirs_through_typed_contracts` | 1 项通过 |
| `just test zeta-tui --lib widgets::list_selection::view::` | 最后绘制调整后 5 项通过 |
| `just test zeta-tui --lib directory_permission_selection_emits_a_revision_bound_server_edit` | 1 项通过 |
| `just test zeta-app-server --lib server::environment_runtime::tests:: -- --test-threads=1` | 21 项通过 |

新快照逐行审阅：等待时显示 Adding directory…；成功时输入清空，显示规范路径，Read files 被选中，No directories 消失。机器未安装 cargo-insta，因此仅将已审阅的确切 `.snap.new` 路径改为 `.snap`，没有批量接受其他工作的快照。

本次是自查。当前全进程 PTY 场景以 `cfg(unix)` 限定，本机未运行；本次不改变终端协议或平台事件解析，验收使用真实 App 状态、真实后端协议和 Ratatui 字符缓冲区。没有运行完整 workspace 测试。

前期编译被同时进行的 OpenAI 工作中间状态阻塞。已补齐协议仍引用的 CustomProvider 类型登记，并修正模型列表中 `DiscoveryCoverage` 的枚举名称；随后协议、目录和 TUI 构建检查通过。其余 OpenAI 修改保留。

后端首次按文件名 `env_runtime_tests::` 过滤未匹配测试（0 项，不计入通过），已改为实际模块路径 `server::environment_runtime::tests::` 重跑。

## 候选标识

验收时 HEAD：`dc97603524fdeee0cafc3541f1cf8c755f590dfa`。本次实现、相关共享文件与生成产物的 SHA-256 见 [source.sha256](source.sha256)；该清单记录共享文件的完整当前内容，因此也包含同一工作区的并行修改，不表示那些修改由本任务完成。证据文件自身不参与代码指纹计算。

- `intent.md`：`0c6ec4e5136a4aa1fa492f3c51accd27986a9b834618a56ee5f8e5017d18a404`
- `spec.md`：`cb3963c58fe3a613618e578922663f02a57bf91de9c9e7344d7a17f59f6ce4b2`

全部 AC 已按上述范围通过；目录快照没有遗留 `.snap.new`，其他工作的快照未接受。构建中的既有未使用代码警告未作为本次改动扩大清理。
