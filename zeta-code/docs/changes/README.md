# 变更记录

- 每条记录表示一次工作；长期功能要求从[功能总览](../capabilities.md)进入。
- 日期取自已有记录或本次整理日期；同日条目不表示先后顺序。未记载的完成日期不补造。
- “已验收”只对记录绑定的范围、版本与环境成立；精确文字或代码差异查 Git 历史。

## 工作索引

| 记录日期 | 工作 | 状态与证据边界 | 对应现行规格 | 对应设计 |
| --- | --- | --- | --- | --- |
| 2026-09-07 | [清理无入口的鼠标接口](2026-09-07-unused-pointer-apis/verification.md) | 已完成；普通构建无警告，701 项测试通过、1 项忽略 | [终端鼠标规则](../spec/terminal.md#鼠标规则) | [TUI](../design/tui.md) |
| 2026-09-08 | [内存诊断配置开关](2026-09-08-memory-diagnostics-config/README.md) | Config 独占启停、Status 只读的自动化通过；Unix PTY 待实测 | [内存诊断](../spec/memory-diagnostics.md) | [采样职责](../design/process-resources.md) |
| 2026-09-07 | [界面语言设置](2026-09-07-interface-language/verification.md) | 已完成本次范围；配置、持久化、生产检查和真实 PTY 场景通过 | [命令面板与设置](../spec/commands.md#设置与快捷键) | [TUI](../design/tui.md) |
| 2026-09-08 | [文档整理](docs-organization.md) | 已完成；链接、格式、导航与历史证据检查通过 | 功能总览、规格拆分与导航 | CLI、TUI、采样职责说明 |
| 2026-09-07 | [目录添加反馈](dir-add/README.md) | 本次 AC 按记录范围通过；Windows 未运行 Unix PTY | [目录与权限](../spec/directories.md) | [TUI 请求与旧结果](../design/tui.md#后台请求与旧结果) |
| 2026-09-08（整理登记） | [OpenAI 配置面板](openai-panel/README.md) | 新版多连接要求尚待验收；保留旧版五条验收 | [供应商与模型](../spec/providers.md) | [TUI](../design/tui.md)、[共享供应商配置](../../../docs/model-provider-config.md) |
| 2026-09-07 | [ChatGPT 初始入口](chatgpt-provider/README.md) | 初始入口有验收，真实设备登录未验证；认证扩展另有记录 | [账户行为](../spec/providers.md#chatgpt-账户) | [共享认证设计](../../../docs/chatgpt-subscription.md) |
| 2026-09-07 | [TUI 功能完整性核对](tui-completeness/README.md) | 27 条要求，未逐项实测；Q-1 至 Q-4 未决 | 见工作内 AC → 规格对照表 | [TUI](../design/tui.md)、[CLI](../design/cli.md)、[采样](../design/process-resources.md) |

## 共享后端的相关工作

| 工作 | TUI 对应规格 | 记录归属 |
| --- | --- | --- |
| ChatGPT 认证复用与维护 | [供应商与账户](../spec/providers.md) | [认证验收](../../../zeta-rs/docs/changes/chatgpt-auth/verification.md)，由共享后端维护 |
| 持续内存诊断 | [内存诊断](../spec/memory-diagnostics.md) | [诊断验收](../../../zeta-rs/docs/changes/memory-diagnostics/verification.md)，由共享后端维护 |

## 保存与查询规则

| 情况 | 做法 |
| --- | --- |
| 同一工作尚未完成 | 继续原目录；只改有变化的内容，不每轮重写四份文件 |
| 目标或范围实质变化 | 追加日期、旧要求、新要求、原因；保留原验收绑定，不把旧 AC 编号直接解释成新要求 |
| 新候选的验证 | 追加候选版本和结果；此前通过、失败与限制继续保留 |
| 工作完成 | 有效要求合入对应现行规格；保留本次目标、变更规格和验收 |
| 后续独立需求 | 新建 `YYYY-MM-DD-工作名/`，链接前次工作；既有目录无需重命名 |
| 查功能现状 | 看 `spec/` 与功能总览，不把历史规格当作现行规则 |
| 查具体差异 | Git 文件历史、提交差异；未提交期间的每次编辑不会自动逐次保存 |

已有记录中的路径纠错可以修复；不得借整理改写原测试命令、版本、结果或代码指纹。完整规则见[开发流程](../../../docs/development-workflow.md)。
