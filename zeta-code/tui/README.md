# `zeta-tui`

`zeta-tui` 是 Zeta 的终端界面，主要负责：

- 接收文字、图片和命令，处理会话、设置与批准面板。
- 通过 App Server 客户端发送请求，把后端状态和流式正文显示到终端。
- 管理终端输入模式、绘制、滚动、鼠标捕获和退出清理。

从仓库根目录运行：

```sh
just zeta
```

[功能现状](../docs/capabilities.md)说明目前能做什么；[交互规格](../docs/spec/interaction.md)说明怎么操作；[架构设计](../docs/design/tui.md)说明各模块为什么这样分工。下面用于定位实现和运行验证。

## 文件与职责

| 要修改什么 | 从哪里开始 |
| --- | --- |
| 启动、事件循环和请求调度 | [start.rs](src/app/start.rs)、[event_loop.rs](src/app/event_loop.rs)、[driver.rs](src/app/driver.rs) |
| 输入、附件、补全和排队发送 | [composer](src/thread/composer)、[submission.rs](src/thread/composer/submission.rs) |
| 批准或回答问题 | [interaction](src/thread/interaction) |
| 正文、执行输出、缓存与滚动 | [transcript](src/thread/transcript) |
| 会话列表、预览、切换和详情 | [sessions](src/sessions) |
| 设置、主题、快捷键 | [config](src/config)、[theme](src/theme)、[keymap](src/keymap) |
| 状态信息和本机资源 | [status](src/status)、[process_resources.rs](../../zeta-rs/memory-diagnostics/src/process_resources.rs) |
| 终端恢复、鼠标和历史输出 | [session.rs](src/terminal/session.rs)、[mouse.rs](src/terminal/mouse.rs) |
| 命令面板共用控件和文字绘制 | [widgets](src/widgets)、[render](src/render) |

Skills、Models、Connectors 和 MCP 各自拥有同名模块；目录授权在 [dirs.rs](src/dirs.rs)。新增功能从对应模块进入，不在 App 里再建一套状态和请求流程。

## 启动与事件循环

CLI 将已初始化的 `AppServerSession` 和 `TuiOptions` 交给 `run`：

1. 校验初始化结果中的命令目录，拒绝非法名称、空描述和内置命令冲突；连接事件流只取一次。
2. 读取配置、主题和启动信息，创建会话及根对话，或恢复指定 Session/Thread。
3. 订阅当前 Thread，安装后端快照和历史分页，组装 App、请求调度器和事件源。
4. 终端输入、后端事件和后台完成事件分别唤醒主循环；主循环更新状态并按需绘制。
5. 退出时结束任务并恢复终端；连接丢失时返回原因及可恢复的会话身份。

输入、请求完成和后端控制事件不能相互长期阻塞。同一资源的写请求保序，不同资源可以并发；中断、批准和回答使用独立控制请求。具体功能解释自己的响应，事件循环只负责转交和调度。

### 公共接口

公开类型和参数以 [lib.rs](src/lib.rs) 为准，模块默认私有。

| 接口 | 用途 |
| --- | --- |
| `client_capabilities` | 向后端声明通知、批准、提问、目录授权和工作协调能力 |
| `TuiOptions` | 指定标题、目录、profile、本地进程身份和恢复信息 |
| `run` | 在已初始化连接上运行一次交互会话 |
| `TuiRecoveryState` | 保存持久化 Session/Thread 身份，不携带连接或待执行请求 |
| `TuiExit` | 区分用户退出、系统终止和连接丢失 |
| `TuiError` | 报告客户端、事件流、关闭和终端错误 |

`with_remote_dir` 只设置远程展示目录，并关闭本地文件补全，避免把远程路径当成本机路径扫描。正文导出仍受先前配置的本机目录约束。`with_profile_root` 启用该 profile 的 TUI 主题目录，其他设置从后端配置读取。

## 输入如何提交

`ChatInput` 管理文字与原子元素，`ChatComposer` 根据任务状态产生 Submit、Queue 或 Steer。补全打开时，Enter/Tab 先处理候选，不能同时发送消息。

| 后端任务状态 | 发送行为 |
| --- | --- |
| 空闲 | Enter 提交新的 Turn |
| 已创建、尚未运行 | Enter 保存到本地 Queue |
| 运行中 | Enter 排队；Ctrl+Enter 单次补充当前任务 |
| 等待批准或回答 | 对应面板处理输入；仍可明确中断 |
| 取消中 | 抑制重复中断 |

运行中 Steer 不能改变 Skill；遇到这种草稿应保留输入，让用户排队或下一轮提交。权限模式的选择用于下一次 Turn，不直接改变当前任务或 Session 权限。

Queue 保存完整草稿，包括图片、长粘贴和绑定的 Skill。恢复编辑保留条目身份，重新排队时替换原条目；不能覆盖非空输入，也不能把编辑后的条目重复追加。发送成功后才移除，失败时保留可恢复内容。当前任务结束后，才将本地队列提交到后端。

任务完成、失败和中断由快照决定。正文增量不能决定终态；其他客户端创建了后续任务时，继续选择最早的未结束 Turn。本地设置或导出错误不改变这一判断。

### 长文本与图片

粘贴先统一换行符。超过 1000 个 Unicode 字符时，以绑定原文的原子占位符显示，提交前展开；重复内容仍有独立身份，删除占位符同时移除绑定。输入区最多显示六行，最近 100 条纯文本提交用于历史召回。

本地 PNG、JPEG、GIF、WEBP 路径和 Ctrl+V 剪贴板图片使用同一附件流程。单图最多 16 MiB；文件列表优先选择可解码图片，否则读取 RGBA 位图并编码为 PNG。草稿显示 `[Image #N]`，删除后重新编号，文字和图片顺序保持不变。

提交前，图片通过后端按 192 KiB 分块上传，或使用共享的远程 URL 安全导入接口，最后发送 `ImageAttachmentRef`。草稿中的本地路径和 data URL 不进入持久化正文、快照或命令收据。

### 命令与补全

| 前缀 | 数据来源与提交方式 |
| --- | --- |
| `/` | 合并本地和后端命令目录；已实现的产品命令进入对应请求流程 |
| `@` | 本地文件搜索与已生效 Plugin 目录；选择结果作为普通文字提交 |
| `$` | 已启用、兼容且无歧义的 Skill 元数据；提交文字及固定版本的 `SkillRef` |

命令补全只替换光标所在的首行命令名，保留参数、图片和粘贴绑定。例如 `/mod provider/model` 补全为 `/model provider/model`。移除命令后的空格后可以重新编辑名称。未知命令或不接受参数却带参数的命令按普通消息处理；已注册产品命令没有实现路径时不能冒充成功。

`/resume`、`/rewind`、`/add-dir`、`/fork`、`/model`、`/theme` 和 `/new` 支持行内参数；产品命令拒绝图片参数。命令回显和结果始终更新同一正文单元，输出格式见[正文输出](../docs/spec/transcript.md)。

文件补全只识别空白分隔的 `@token`，不处理邮箱中的 `@`。扫描遵守 Git 忽略规则、不跟随符号链接，并跳过 `.git`、`.zeta`、`node_modules` 和 `target`；结果按匹配分数与路径稳定排序，最多 50 项。请求同时校验查询文本和版本，关闭补全后释放搜索句柄。

TUI 不扫描 Skill 正文；完整 `SKILL.md` 由后端在接受任务后按需加载。`skills/changed` 刷新目录与候选；Plugin 安装或变更也刷新相关 Skill 与打开的 Connector 面板。

## 面板怎样接入后端

通用导航、搜索和返回行为见[面板键表](../docs/spec/commands.md#每个命令面板)。下面只列会影响请求实现的区别。

| 功能 | 接入要求 |
| --- | --- |
| 会话管理 | `/agents` 与 `/sessions` 打开同一管理器；`/subagents` 进入当前会话的 Thread 切换区 |
| 回退 | `/rewind` 或空输入下 500 ms 内连续 Esc 创建子 Thread，继承目标之前已结束的任务，原 Thread 不变 |
| 归档 | 归档当前会话成功后才创建新会话；归档列表的恢复不重跑已结束任务 |
| Skills | 目录只展示元数据；Manage 使用带来源的 Skill 身份与配置版本修改启用状态，随后重读 |
| Connectors | 支持设备码授权和断开；API key 与浏览器回调 OAuth 连接仍通过 Desktop 设置完成 |
| 目录 | 添加目录和授予访问权限分别处理；使用 Session RPC 与权限版本，不写入 profile 设置 |
| 设置和模型 | 带预期配置版本保存；各功能只接收自己需要的字段 |
| 批准和提问 | 只有后端选中且订阅该 Thread 的连接可以回答；禁止重复提交，超时由后端处理 |

Connector 操作见 [request.rs](src/connectors/request.rs)：设备码复制到剪贴板后打开验证网址，按服务端间隔轮询；失败时取消授权流程。目录版本和连接代次用于拒绝过期操作。

配置保存替换完整 `[tui]` 表，因此必须保留其他 TUI 设置。API key 只通过专用凭据接口保存，不进入普通配置或展示状态。快捷键候选先完成全量校验，保存失败时保留上一份有效规则；面板基础键与用户可重绑的应用动作范围见[快捷键规格](../docs/spec/commands.md#快捷键声明与保存)。

`/config` 的 Providers 页提供独立的 `ChatGPT subscription` 入口，可查看 Zeta 账户和方案、启动设备码登录、取消登录或退出。验证地址与一次性代码显示在账户页；Esc 返回 Providers，待完成登录仍可重新进入查看和取消。TUI 使用共享账户接口，后端有 Codex 时只读复用，无 Codex 时负责续期和重新登录；缺失时生成兼容的 auth.json。复用模式断开不会退出 Codex；自管模式登出清除认证。重新连接仍有效的已有凭据无需浏览器。[认证存储与验收](../../zeta-rs/docs/changes/chatgpt-auth/verification.md)。

资源采样由可见状态行项目和 Processes 页共同决定；没有需求时停止采样。关闭 Git 显示只停止状态行专属工作，不能停止 ChangeTurn 的目录跟随。统计定义见[进程资源设计](../docs/design/process-resources.md)。

## 正文更新与容量

正文保持协议中的原始输出，ANSI 颜色转换与制表符展开只在绘制时处理；每个 tab 显示为四个空格。代码块使用共享语法高亮；流式高亮只追加已换行的完整代码行，未知语言、解析失败或超限时保留原文。

| 对象 | 当前限制 |
| --- | --- |
| 一批流式更新 | 最多 256 个身份、1024 次更新、1 MiB 正文 |
| 临时正文 | 单条 256 KiB，最多 1024 个身份 |
| 跨 Thread 界面状态 | 最近 32 条 Thread；切走时释放重型绘制缓存 |
| 进程资源历史 | 最多 301 个本机内存合计读数 |
| 连续更新重绘 | 首次请求后 16 ms 内；新请求不延后期限，输入可立即绘制 |

只有同一会话、对话、持久化序号和流身份，且游标、正文版本连续的完整更新可以合并。提交、删除、清空、输入和控制事件结束当前批次。重复更新忽略；缺口或流切换则丢弃不可信的临时正文并重读快照。

执行输出按调用身份归组，稳定标识取组内第一个 `ToolCallId`。预览、展开和详情共用有界数据；“完整详情”指 TUI 获得的完整保留内容，必须保留上游省略标记。不能从文字猜测协议未提供的最终时长或退出码。

初始快照从最近 50 个 Turn 开始，订阅随后自动读完历史分页。已定稿前缀只提交到终端历史一次，普通画面继续绘制可变尾部；正文浏览组合两段。滚动位置用单元身份和行偏移保存，不能依赖屏幕行号。

## 产品支持边界

正文支持普通折行和 fenced code block 高亮，尚未实现完整 Markdown、可点击 Markdown 链接或任意 HTML 展示。增强鼠标只处理可见的详情与补全覆盖浮层；占据布局高度的区域不接收 TUI 鼠标操作，终端自身的历史、文字选择和复制仍由终端处理。Vim 只改变输入框编辑。

`/export [relative-path]` 导出当前已加载正文，路径限制在本机工作目录内，不能覆盖已有文件。Ctrl+O 复制最后一条 Agent 回复。

断线后，TUI 丢弃旧连接的待执行请求和操作，只返回持久化会话身份。本地和远程 CLI 在 30 秒窗口内重连；失败时分别给出 `zeta resume SESSION_ID THREAD_ID` 或 `zeta remote connect ... --resume SESSION_ID THREAD_ID`。正常服务端关闭和协议错误不进入传输重试。

## TUI 主题文件

交互事实由[交互契约](../docs/spec/interaction.md#交互状态)定义；内置颜色、字符与绘制优先级由[样式契约](../docs/spec/styles.md)定义。本节只拥有用户主题文件格式。

TUI 设置保存在 `<profile>/config.toml` 的根级 `[tui]` 表：

```toml
[tui]
theme = "graphite"
mouseInteractions = true
inputMode = "standard"
showGitChangesAsDiff = false
```

目录权限不属于 TUI profile 设置，只保存在对应 Session。用户主题内容保存为 `<profile>/zeta-code/themes/*.json`。每个文件最多 1 MiB；目录最多读取 128 个常规 JSON 文件；`id` 必须是小写 kebab-case，`label` 为 1–80 个已去除首尾空格的字符，`appearance` 只能是 `dark` 或 `light`，`colors` 最多覆盖 64 项且颜色必须是 `#RRGGBB`。未知字段、未知颜色名、重复/保留 ID 和不支持的版本都会使该主题文件单独失效。

```json
{
  "schemaVersion": 2,
  "id": "graphite",
  "label": "Graphite",
  "appearance": "dark",
  "colors": {
    "background": "#101010",
    "quickViewBackground": "#303030",
    "transcriptJumpBackground": "#414141",
    "userMessageBackground": "#252525",
    "actionForeground": "#58a6ff",
    "focus": "#8b80f9",
    "hoverBackground": "#25233a",
    "hoverForeground": "#f0edff"
  }
}
```

可覆盖字段为 `accent`、`accentSurfaceBackground`、`accentSurfaceForeground`、`actionForeground`、`background`、`border`、`chatInputChrome`、`danger`、`disabledForeground`、`focus`、`foreground`、`function`、`hoverBackground`、`hoverForeground`、`insertedBackground`、`insertedMarker`、`keyword`、`muted`、`pressedBackground`、`pressedForeground`、`quickViewBackground`、`removedBackground`、`removedMarker`、`selectionBackground`、`selectionForeground`、`screenSelectionBackground`、`screenSelectionForeground`、`string`、`success`、`transcriptJumpBackground`、`type`、`userMessageBackground`、`variable` 与 `warning`。未写字段继承所选 `appearance` 的内置调色板；该格式不接受图形界面 token、别名、透明色或颜色变换。

## 终端生命周期

当前使用终端主屏幕。`TerminalSession::open` 先检测终端，获取模式、查询背景色，再创建 Ratatui 终端并清理当前绘制区域。

模式按以下顺序获取：原始输入模式 → 主屏幕准备 → 粘贴事件 → 焦点上报。只有增强鼠标开启且存在可见覆盖浮层时才启用鼠标捕获；事件循环还会检查鼠标是否落在浮层内，关闭开关后的残留鼠标事件也会被忽略。

`TerminalModeGuard` 记录每一步是否成功。任一步失败或退出时，逆序关闭鼠标、焦点上报、粘贴事件，结束当前屏幕并关闭原始输入模式。显式恢复可重复调用，Drop 再次清理不会重复操作；退出或挂起时还要重置光标颜色并显示光标。

鼠标边界回归见 [event_loop_tests.rs](src/app/event_loop_tests.rs)。在窗口至少 40×12 的真实 PTY 中运行 `just test zeta-tui --lib real_terminal_mouse_handoff -- --ignored --nocapture --test-threads=1`，可验证固定面板不捕获、补全开启捕获、关闭浮层或增强开关后释放捕获。该场景不替代各终端自身的选文与复制兼容性验证。

Ctrl+Z 在 Unix 上先恢复终端，再发送 SIGTSTP；`fg` 后重新获取模式并重绘。SIGINT/SIGTERM 进入正常事件循环退出路径。新增模式时同时修改获取标记、逆序清理和 [session_tests.rs](src/terminal/session_tests.rs) 中的部分失败测试。

## 修改 Welcome 宠物

只编辑 [pet.sprite](assets/welcome/pet.sprite) 中的终端格、帧和动作；[build.rs](build.rs) 在构建时校验并嵌入数据。规格见[Welcome 宠物](../docs/spec/welcome-pet.md)。

```sh
just pet
just pet frames
just pet click
```

这些命令分别预览静止帧、全部帧和点击动作。动作资源与独立预览已具备，Welcome 点击播放尚未接入。

## 测试与支持边界

在仓库根目录执行受影响的检查：

```sh
just check zeta-tui
just test zeta-tui
just test zeta-cli --test tui_real_scenarios
```

功能模块的测试检查状态、请求和完成结果；App 测试检查跨功能路由、优先级和退出；真实 PTY 场景检查完整 CLI/TUI 操作。上述命令是执行入口，不是本次通过记录。

渲染测试使用 Ratatui 字符缓冲区与 `insta`；状态、协议和副作用仍需独立断言。固定尺寸，规范化动态路径和身份，逐项审查 `.snap.new` 后再接受，具体操作见[字符快照测试](../../.agents/skills/zeta-code-snapshot-testing/SKILL.md)。

共享完整配置快照使用 `test_support::empty_config_snapshot`，测试只修改自己关心的字段。直接构造 `ThreadItem` 时显式填写各字段，避免测试助手隐藏实际业务要求。

## 终端历史兼容性验证

`terminal-detection` 识别终端身份、颜色和复用器，不证明回滚区行为。完整输出是 TUI 的职责；保留上限由终端的回滚行数决定。以下保留先前文档记录的 2026-09-07 验证范围，本次整理没有重新执行这些验证。表中结论只适用于记录中的协议、引擎、版本和环境，不证明当前候选或所有宿主均通过。

| 检测器中的终端或复用器 | 验证状态 |
| --- | --- |
| VsCode | xterm.js 6.0.0 在 Chromium 中通过包含旧交互画面的协议回放、缩放和滚轮断言；未完成 VS Code 整个应用的端到端验证 |
| WezTerm | 旧版整屏追加协议曾在 Windows 版 20240203-110809-5046fc22 的 ConPTY 路径通过；当前单元前插协议尚未重新实测 |
| WindowsTerminal | 先前记录未实测；启动别名不构成通过证据 |
| AppleTerminal、Iterm | 先前记录未实测；需补具体 macOS 终端验证 |
| Ghostty、Warp、Kitty、Alacritty | 先前记录未实测；需补对应终端验证 |
| Konsole、Gnome、Vte | 先前记录未实测；需补对应 Linux 终端验证 |
| Tmux、Zellij | 未实测；必须验证复用器与外层终端的组合，不能只验证外层 |
| Dumb、Unknown | 没有兼容性承诺；未知身份与缺少交互能力不能等同于已验证的终端 |

这轮验证发现两类边界问题：先绘制整批内容再集中换行会在 WezTerm 的 Windows ConPTY 路径丢失每批最后一行；清空并滚动整个主屏幕则会把 Welcome、补全、输入框和状态栏错误提交到回滚区。历史输出因此逐个借用顶行写入定稿单元并立即恢复原顶行，普通画面只绘制尚未定稿的尾部。协议测试会先放入交互内容，再断言回滚区只出现目标正文。只检查消息去重状态或最终画面都无法发现这一问题。

可重复验证使用同一份生产输出，避免为不同终端另写一套模拟输出算法：

1. 在仓库根目录生成协议样本：PowerShell 设置 `$env:ZETA_TUI_HISTORY_FIXTURES = "$PWD/output/terminal-history"`，然后执行 `just test zeta-tui --lib history_compatibility_corpus`。完成后移除这个环境变量。测试只替换终端尺寸查询，保留正式的 Fullscreen 与 Crossterm 输出路径。
2. 在待测终端调整到样本尺寸，执行 `python zeta-code/tui/tests/terminal_history.py replay output/terminal-history 80x24-120.ansi`，保持程序等待输入。导出包含回滚区的 UTF-8 纯文本，执行 `python zeta-code/tui/tests/terminal_history.py verify output/terminal-history 80x24-120.ansi --capture <导出文件>`。WezTerm 可使用 `wezterm cli get-text --pane-id <编号> --start-line -10000` 导出。再运行 `80x24-1.ansi` 验证短内容。
3. 验证 xterm.js 时，将所测版本的 `xterm.js` 与 `xterm.css` 放到样本目录，用 `python -m http.server 8779 --bind 127.0.0.1 --directory output/terminal-history` 提供本地页面。用 Playwright CLI 打开该地址，再运行 `run-code --filename zeta-code/tui/tests/terminal_history.js`。脚本读取全部六组样本，检查旧 shell 内容、所有正文标记恰好一次且顺序正确、缩放后内容完整，以及关闭鼠标捕获时滚轮只移动终端回滚区且不产生按键输入。

样本覆盖 80×24、40×5、12×3、120×40、12×1，以及不足一屏的短内容；长内容有 120 个含中文和 emoji 的标记。验证结果只覆盖样本中实际执行的行为。面板增强开关、会话恢复分页、流式定稿与去重另外由对应的应用状态和协议测试负责，不能用这些样本替代。
