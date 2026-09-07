# TUI 功能完整性：验收记录

**当前不能确认 TUI 功能完整。** 27 条要求已登记源码或测试线索，但本轮没有执行产品测试，尚无任何一条可以标为实际验收通过。四项范围问题也仍待确定。

## 本轮核对范围

- 输入：[目标 v1](intent.md)、[规格 v1](spec.md)，2026-09-07。
- 初次源码核对基线：`f173facf5f08e7f0f0611caf5dc64be6a5d03e56`。核对期间仓库切入包含正文重构提交 `67603c26280be2dd401b0f8ca8449278ada5a84e` 的新版本；正文、输入路由和终端实现均有变化。下表原有源码线索保留为检查入口，受影响行为必须按新候选重新核对，不能沿用旧基线的完成判断。
- 环境与方法：当前 macOS 工作区，阅读源码和已有测试断言，检查文档关联；未启动 TUI、未调用真实模型或外部授权服务。
- 审查：本任务自查。下面的“已核对”只描述实际阅读范围；测试文件存在不代表测试通过。

## 逐项核对

所有编号对应[功能要求](spec.md)。表中保留尚未覆盖的部分，后续执行后将命令、环境和结果填回对应行。

| 要求 | 实现核对情况 | 当前证据与剩余检查 | 实测结果 |
| --- | --- | --- | --- |
| AC-1 启动与连接 | 入口待核实 | [启动](../../../tui/src/app/start.rs)、[真实场景](../../../cli/tests/tui_real_scenarios.rs)；需补首次连接失败与空界面验收 | 尚未验证 |
| AC-2 恢复已有会话 | 有恢复实现与测试 | [恢复测试](../../../tui/src/sessions/active_tests.rs)包含指定对话和分支身份；还需检查无效身份的产品提示 | 尚未验证 |
| AC-3 文字、粘贴与补全 | 入口待核实 | [编辑测试](../../../tui/src/thread/composer/input/editor_tests.rs)、[命令补全测试](../../../tui/src/thread/composer/input/slash_commands_tests.rs)；需覆盖中文、长粘贴、取消和空提交 | 尚未验证 |
| AC-4 图片输入 | 已核对部分断言 | [附件测试](../../../tui/src/thread/composer/input/attachments_tests.rs)断言占位符、大小上限和删除重编号；未核对完整上传失败恢复 | 尚未验证 |
| AC-5 提交、排队与补充 | 已核对发送分支 | [提交实现](../../../tui/src/thread/composer/submission.rs)区分 Enter、Ctrl+Enter 和候选状态，并拒绝运行中更换 Skill | 尚未验证 |
| AC-6 队列编辑 | 有相关测试入口 | [队列测试](../../../tui/src/thread/queue_tests.rs)、[真实场景](../../../cli/tests/tui_real_scenarios.rs)；需核对非空草稿及发送失败不丢内容 | 尚未验证 |
| AC-7 执行状态与错误 | 入口待核实 | [Thread 状态](../../../tui/src/thread)、[真实故障场景](../../../cli/tests/tui_real_scenarios.rs)；需逐类检查终态和重复错误 | 尚未验证 |
| AC-8 中断与请求优先级 | 有真实场景入口 | [CLI 场景](../../../cli/tests/tui_real_scenarios.rs)包含中断 HTTP 流；普通查询并发和重复中断仍需核验 | 尚未验证 |
| AC-9 批准与拒绝 | 有交互测试入口 | [批准测试](../../../tui/src/thread/interaction/approval_tests.rs)、[真实文件工具场景](../../../cli/tests/tui_real_scenarios.rs)；超时及后端拒绝需另验 | 尚未验证 |
| AC-10 回答问题 | 有交互测试入口 | [提问测试](../../../tui/src/thread/interaction/query_tests.rs)包含逐页回答、自定义输入和粘贴；提交失败恢复仍待核对 | 尚未验证 |
| AC-11 会话分组 | 已核对默认状态 | [管理器](../../../tui/src/sessions/manager.rs)仅默认收起 Archived；[应用测试](../../../tui/src/app/session_manager_tests.rs)包含分组导航和展开收起 | 尚未验证 |
| AC-12 只读预览与返回 | 已核对部分断言 | [预览测试](../../../tui/src/app/session_manager_tests.rs)明确断言不修改草稿、Esc 返回和丢弃旧结果；尚未实际操作验证 | 尚未验证 |
| AC-13 切换与详情刷新 | 已核对部分实现 | [详情实现](../../../tui/src/sessions/details.rs)、[详情测试](../../../tui/src/sessions/details_tests.rs)、[订阅测试](../../../tui/src/thread/subscription_tests.rs)；跨会话并发结果仍需完整验证 | 尚未验证 |
| AC-14 归档、恢复与删除 | 有相关测试入口 | [Session 测试](../../../tui/src/sessions/active_tests.rs)、[归档列表场景](../../../tui/src/app/session_manager_tests.rs)；删除失败及选中对象正确性待验 | 尚未验证 |
| AC-15 分支与回退 | 入口待核实 | [回退请求](../../../tui/src/thread/rewind/request.rs)、[恢复测试](../../../tui/src/sessions/active_tests.rs)；需核对历史边界和原对话不变 | 尚未验证 |
| AC-16 长输出与滚动 | 实现已更新，待重新核对 | [正文测试](../../../tui/src/thread/transcript/view/render_tests.rs)包含折行和滚动锚点；截断详情、窄窗口与所有输出类型需实测 | 尚未验证 |
| AC-17 终端历史 | 有协议与渲染测试 | [终端测试](../../../tui/src/terminal/session_tests.rs)、[历史协议测试](../../../tui/src/terminal/history_protocol_tests.rs)；真实宿主范围依赖 Q-2 | 尚未验证 |
| AC-18 复制与导出 | 已核对导出限制 | [导出](../../../tui/src/host/transcript_export.rs)使用禁止覆盖的文件创建方式并校验目录；[导出测试](../../../tui/src/host/transcript_export_tests.rs)；剪贴板需实测 | 尚未验证 |
| AC-19 设置保存 | 已核对部分版本检查 | [配置请求](../../../tui/src/config/request.rs)、[快捷键测试](../../../tui/src/keymap/settings_tests.rs)；需验证模型、主题、冲突和重新读取 | 尚未验证 |
| AC-20 面板输入与返回 | 有应用场景入口 | [实际面板场景](../../../cli/tests/tui_real_scenarios.rs)、[通用控件](../../../tui/src/widgets)；逐面板焦点和录制隔离待验 | 尚未验证 |
| AC-21 Skill 与补全 | 有实现和测试入口 | [Skill 候选测试](../../../tui/src/thread/composer/input/completion/skill_tests.rs)、[文件搜索测试](../../../tui/src/thread/composer/file_search_tests.rs)、[Skills](../../../tui/src/skills)；需贯通启用到实际提交 | 尚未验证 |
| AC-22 Connector 与 MCP | 已核对部分请求流程 | [Connector 请求](../../../tui/src/connectors/request.rs)已有设备码授权与断开；[MCP](../../../tui/src/mcp)；真实授权和失败状态未验证 | 尚未验证 |
| AC-23 目录权限 | 入口待核实 | [目录功能](../../../tui/src/dirs.rs)；会话隔离、撤销、冲突和拒绝需连同后端逐项验收 | 尚未验证 |
| AC-24 断线恢复 | 已核对恢复身份 | [断线处理](../../../tui/src/app/recovery.rs)区分传输、正常关闭和协议错误；[公开恢复类型](../../../tui/src/lib.rs)不携带待执行请求；完整重连链待验 | 尚未验证 |
| AC-25 退出与挂起 | 有生命周期测试 | [终端测试](../../../tui/src/terminal/session_tests.rs)包含逆序恢复、部分获取失败、重复恢复和挂起；实际信号与后台任务清理待验 | 尚未验证 |
| AC-26 键盘、鼠标与窄窗口 | 有局部测试入口 | [鼠标测试](../../../tui/src/terminal/mouse_tests.rs)、[字符选择测试](../../../tui/src/terminal/screen_selection_tests.rs)；全流程键盘、无颜色和选定尺寸仍待验 | 尚未验证 |
| AC-27 状态与资源采样 | 有生命周期测试入口 | [采样测试](../../../../zeta-rs/memory-diagnostics/src/process_resources_tests.rs)包含按需启停；[状态页](../../../tui/src/status/panel.rs)；合计口径与不可用状态仍待验 | 尚未验证 |

## 已知缺口与范围问题

| 问题 | 当前结论 | 对完成判断的影响 |
| --- | --- | --- |
| Q-1 · Markdown | 当前支持边界记录正文链接尚不可点击；尚未逐项审计表格及其他 Markdown 行为 | 先确定需要的呈现范围，再补对应要求与验证 |
| Q-2 · 终端组合 | 既有[兼容性记录](../../../tui/README.md#终端历史兼容性验证)仅覆盖列明的历史环境；CLI PTY 测试文件有 `cfg(unix)` 限制 | 不能把 Unix 场景或单个引擎通过外推为所有平台通过 |
| Q-3 · 未发送内容恢复 | 恢复类型只携带 Session/Thread 身份，不携带本地草稿或队列 | 需要确定持久化要求；目前无法宣称未发送内容可跨断线恢复 |
| Q-4 · 宠物点击播放 | [Welcome 绘制](../../../tui/src/app/welcome/pet.rs)仍取 `idle()`，动作资源和预览不能证明点击已接入 | 明确是否本轮必做；若是，登记实现与验收步骤 |

正文重构已有新的实现提交，但本轮没有验证它是否满足独立规格；内存诊断已由独立变更接入，不能把其测试等同于本轮验收。发现新的必要漏项时，先补规格，再在本表增加记录。

## 文档检查

四份正文与首页链接已建立。2026-09-07 的文档自查结果：27 条 AC 在规格和验收中各出现一次，计划覆盖全部编号，4 项范围问题对应一致；八份相关文档的 150 个本地链接、章节锚点和空白检查通过。新增文档没有模板占位符或冲突标记。相关路径的 `git diff --check` 通过；这些结果只证明文档关联有效。

本次只修改 Markdown，没有新增或修改产品测试，也未运行 Rust 构建。上表保留“尚未验证”；后续按[计划](plan.md#运行验证)执行，并记录实际候选代码、环境、命令、失败原因和复验结果。
