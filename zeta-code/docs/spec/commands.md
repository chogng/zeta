# 命令面板与设置

- 本文维护命令入口、主动作、返回路径，以及设置和扩展能力的操作要求。
- 通用列表、搜索和页签键位见[交互规则](interaction.md#统一按键与界面键表)。
- 目录表单见[目录与权限](directories.md)，OpenAI 表单与账户见[供应商与模型](providers.md)。

## 每个命令面板

Config 总页的“推荐合并处理”默认勾选；取消后禁用同一面板的 Issues Tab，保留分析模型。模型的选择与保存见 [Issue 配置](issues.md#合并推荐配置)。

- 下表的“列表导航”均指[通用键表](interaction.md#统一按键与界面键表)的 ↑↓/jk、Home/End、PageUp/PageDown；同级面板的 Esc 都关闭面板并恢复输入。
- 表内另写出的返回位置优先。
- 表中“无”表示该键没有业务动作，不传递给背景。

| 界面 / 入口 | 导航与搜索 | 页签 / 左右键 | Enter | Space | Esc |
| --- | --- | --- | --- | --- | --- |
| Issues `/issue` | 列表导航；`/` 过滤已加载 issue，`n` 下一页，`r` 刷新 | Tab/Shift+Tab 切 Open / Closed；上下键到达底部起点动作 | 阅读详情或创建一个合并处理的会话 | 多选 | 返回 |
| Issue PR `/pr` | 选择创建方式；`i` 查看发布范围，`r` 刷新 | 单页 | 提交任务变更、推送并创建所选 PR | 无 | 返回 |
| Help `/help` | 列表导航；`/` 搜索帮助 | Tab/Shift+Tab 切 Shortcuts、Commands、Custom commands | 无业务动作 | 无 | 关闭 |
| Directories `/add-dir` | 列表导航；`/` 输入目录路径 | 单页 | 输入框中添加目录；列表中移除目录或更改权限 | 无 | 输入框中返回列表；等待添加时关闭面板；列表中关闭 |
| Model `/model` | 列表导航；`/` 搜索模型 | 单页 | 应用模型 | 无 | 关闭 |
| Theme `/theme` | 列表导航；无搜索；编号只是标签 | 单页 | 应用主题，或进入 Custom color theme | 无 | 主列表关闭；自定义主题列表返回主列表 |
| Config `/config` | 列表导航；`/` 搜索设置 | Tab/Shift+Tab 切分类；条目左右调整值 | 更改值或进入字段编辑 | 与 Enter 相同 | 关闭；字段编辑内取消并返回设置列表 |
| Keymap `/shortcuts` | 列表导航；`/` 搜索快捷键 | Tab/Shift+Tab 切分类 | 主列表打开动作菜单；菜单执行绑定编辑或开始录制 | 无 | 菜单返回主列表；录制取消后回菜单；主列表关闭 |
| Connectors `/connectors` | 列表导航；`/` 搜索连接器 | Tab/Shift+Tab 切分类 | 连接或断开当前项 | 无 | 关闭 |
| MCP `/mcp` | 列表导航；`/` 搜索服务器 | Tab/Shift+Tab 切分类 | 切换启用状态 | 与 Enter 相同 | 关闭 |
| Skills `/skills` | 列表导航；`/` 搜索技能 | Tab/Shift+Tab 切分类 | 切换当前技能或执行 Manage 条目 | 与 Enter 相同 | 关闭 |
| Sessions `/resume` | 列表导航；`/` 搜索保存的会话 | 单页 | 恢复并进入所选会话 | 无 | 关闭 |
| Rewind `/rewind` | 列表导航；`/` 搜索检查点 | 单页 | 回退到所选检查点 | 无 | 关闭 |
| Status line `/statusline` | 列表导航；无搜索 | 单页 | 切换状态行条目 | 与 Enter 相同 | 关闭 |
| Startup `/startup` | 列表导航；无搜索 | 单页 | 无业务动作 | 无 | 关闭 |
| Status `/status` | 阅读导航；无搜索 | Tab/Shift+Tab 或左右切 Thread / Processes；各页保留滚动位置 | 无 | 无 | 关闭 |

## 设置与快捷键

| 场景 | 行为 |
| --- | --- |
| Config 字段编辑 | 文字和粘贴只编辑当前字段；Enter 保存，Esc 取消；主列表单键不生效 |
| Keymap 录制 | 接受一次按键或连续两段键序列；Esc / Ctrl+C 取消后回菜单 |
| 录制普通字母 | `j/k/i/p/空格` 都是待录制数据，不触发背景操作 |
| 列表中的 Ctrl+C | 等价于 Esc 返回，不中断背景任务 |
| 设置、模型、主题和快捷键保存 | 成功后界面与重新读取的值一致；无效输入、版本冲突或保存失败显示原因，不覆盖其他设置 |
| Memory diagnostics | 仅在 Config 中启停并持久化；Status 只读显示诊断状态，不提供第二个开关 |
| Config 根页面 | 显示 Enhanced TUI、Vim mode、Memory diagnostics、Git 差异展示和 Language 等可操作设置；模型、审批模型和供应商数量不在这里重复展示 |
| Language | Enter、Space 或右方向选择下一种语言，左方向选择上一种；只支持 English、日本語、中文、Français，保存成功后立即刷新 Config 根页面 |

界面语言保存在当前 profile 的 `[tui].language`。语言选择和 Config 根页面文字通过 TUI 的类型化文案接口解析；供应商名称、语言服务器标识、模型回复、代码和用户内容保持原文。无效语言值使本次 TUI 配置读取失败，不静默改用默认值。

### 快捷键声明与保存

- 默认按键、面板动作名称和 HitBar 组合统一在 `zeta-code/tui/src/keymap/bindings.rs` 声明。
- `Keybinding` 同时提供按键匹配与提示标签；列表接收具体面板的绑定，导航、搜索、页签、会话管理、审批、提问、队列和正文详情读取同一入口。
- 面板仍拥有焦点、按下与重复事件策略以及动作执行。
- 文字编辑与 Vim 输入语义留在输入组件。

- 用户覆盖规则继续保存在当前 TUI profile 的 `config.toml`，使用有序的 `[[tui.keybindings]]` 数组，不新增并行 JSON 文件。
- 规则与 `[tui]` 共用设置编辑和 revision 生命周期，TUI 负责命令、条件、顺序与完整候选校验，持久化服务负责 revision 冲突检查。
- 内建声明不写入用户配置，提示文案也不作为用户配置保存。

当前只有应用命令目录中的动作支持用户覆盖；面板绑定已经统一为源码声明，但没有因此开放为 TOML 命令。新增可配置面板动作时，必须同时定义动作身份、焦点条件、冲突规则以及实际绑定对应的提示，不能仅新增一个配置字段。

## 扩展能力

| 能力 | 操作与失败处理 | 共享约定 |
| --- | --- | --- |
| Skill 与输入补全 | 候选插入保留参数及附件；提交身份匹配启用状态；旧查询不覆盖新输入，不可用项不能冒充可执行 | [Skills](../../../docs/skills.md) |
| Connector | 查看连接状态，执行支持的连接或断开动作；设备授权和断开失败显示原因，需要桌面完成时明确提示 | [Connectors](../../../docs/connectors.md) |
| MCP | 查看并修改启用状态；失败保留已生效状态并显示原因 | [MCP](../../../docs/mcp.md) |

## 相关记录

- [功能完整性核对](../changes/tui-completeness/README.md)：AC-19 至 AC-22。
- [功能现状](../capabilities.md)：实现状态与验证限制。
