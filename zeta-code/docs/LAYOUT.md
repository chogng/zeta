# Zeta Code TUI 布局与部位名称

> 这是一份定位界面问题用的词典。讨论 UI 时优先使用本文中的中文名称；括号里的英文名与代码名称用于搜索实现。
>
> 整页区域由 [`app/layout.rs`](../tui/src/app/layout.rs) 分配，绘制顺序由
> [`app/frame.rs`](../tui/src/app/frame.rs) 决定。架构、交互和视觉样式分别见
> [`tui.md`](tui.md)、[`tui-interaction.md`](tui-interaction.md) 与 [`styles.md`](styles.md)。

## 1. 先分清当前页面

整个终端可见范围叫 **屏幕**（Screen）。当前只需先区分两种屏幕：

| 中文名称 | 代码名称 | 怎么认 |
| --- | --- | --- |
| Session 页面 | `TerminalScreen::Session` | 能看到对话正文、输入框和状态行 |
| Session 管理页面 | `TerminalScreen::Manager` | 能看到 Welcome 和按状态分组的 Session 列表 |

不要只说“主页面”或“列表页”。直接说“Session 页面”或“Session 管理页面”。

## 2. 行首标记栏

正文、输入和选择列表中的内容行通常把最左侧两个终端格保留为 **标记栏**（marker gutter）：第一个格显示标记，第二个格负责把标记与正文隔开。正文因此通常从第三个终端格开始。标记栏是跨组件共用的横向对齐规则，不是一个独立的页面区域。

“标记栏”比“装饰列”更准确，因为这里的字符会表达角色、状态、层级或当前操作目标，不只是视觉装饰。终端的一“列”只能容纳一个显示格，所以描述整个两格结构时用“栏”，描述其中某一格时才用“列”。

```text
标记栏  正文
┌──┬───────────────────────────────────────────────────────┐
│> │ 用户输入                                              │
│● │ Agent 或系统输出                                      │
│> │ 键盘当前项                                            │
│  │ 上一行正文的续行                                      │
└──┴───────────────────────────────────────────────────────┘
```

| 标记 | 当前用途 | 常见位置 |
| --- | --- | --- |
| `> ` | 用户输入、可输入位置或键盘当前项 | 输入框、历史用户消息、已结束的本地命令、审批、提问和各种选择列表 |
| `● ` | Agent、系统或正在执行的输出；颜色再表达具体状态 | 正文单元、Agents 中当前查看的 Thread |
| `○ ` | 非当前项或非活动项 | Agents 中其他 Thread |
| 两个空格 | 此行没有独立标记，但正文仍与其他行对齐 | 多行正文的续行、未选中的列表项 |

有层级关系的内容可以把标记栏扩展为连接前缀，例如展开详情使用 `└─ `，其中 `└─` 表达它属于上一条输出，后面的空格仍负责分隔正文。Session 管理页面使用静态分组标题，下面的 Session 行在标记栏之后显示状态图标；标题与内容的横向错位表达归属，不再提供分组折叠标记。这些都是有语义的层级表达，不应为了强行限制在两个格内而丢掉层级信息。

输入框边线、空行以及 Goal、Plan、Queue、状态行等整行辅助信息不需要伪造标记来填满标记栏；只有需要表达角色、状态、层级或当前操作目标的内容行才使用它。标记栏不是独立布局区域，由各组件按上述前缀和对齐规则分别绘制；定位问题时仍统一称为“标记栏”。

## 3. Session 页面

下面是各区域都出现时的纵向顺序。实际界面会隐藏没有内容的区域，所以它们不一定同时可见。

```text
┌──────────────────────────────────────────────────────────┐
│ 正文区 Transcript                                        │
│   ├─ Welcome（空 Session 时）                            │
│   └─ 正文单元 Transcript Cell                            │
│            Jump to bottom (click) ↓（离开最新位置时）    │
│                                                          │
├──────────────── Goal 行（有目标时）─────────────────────┤
├──────────────── Plan 行（有进行中计划时）───────────────┤
├──────────────── Queue 区（有排队消息时，最多显示 3 条）─┤
│ Query 面板（Agent 提问时，位于输入位置上方）             │
│ 顶部提示 TopTip（固定一行，提示为空时留空）              │
├──────────────────────────────────────────────────────────┤
│ 输入位置 Composer Position                               │
│   通常显示 ChatInput；也可能被 Approval 或命令面板替换    │
├──────────────────────────────────────────────────────────┤
│ 输入位置下方两行                                          │
│   常态显示 StatusLine；交互时首行留空                     │
│   末行显示操作提示 KeyHints                                │
│                                                          │ ← 条件性间隔
│ Agents AgentThreadSwitcher（有被委托 Agent 时）          │
└──────────────────────────────────────────────────────────┘
```

### 固定区域词典

| 推荐名称 | 代码名称 | 内容与边界 |
| --- | --- | --- |
| 正文区 | `transcript` / `ChatHistory` | 对话内容占据的主要滚动区域；空 Session 的 Welcome 也画在这里 |
| 正文单元 | `TranscriptCell` | 正文区内的一条用户消息、Agent 回复、命令或工具结果；“某条消息”要定位到这一层 |
| 回到底部控件 | `JumpToBottom` | 仅在正文离开最新位置时覆盖在正文区最后一行中央；它紧邻 `TopTip` 上方，但仍属于正文区，不占用或改写 `TopTip` |
| Goal 行 | `goal` | 一行 `Goal: …`；无目标时高度为 0 |
| Plan 行 | `plan` | 一行 `Plan 已完成数/总数: 当前步骤`；计划结束后隐藏 |
| Queue 区 | `queue` | 输入在当前 Turn 结束后再发送时，显示排队消息；普通输入框不属于这里 |
| 提问面板 | `request` / `Query` | Agent 要求用户选择或输入答案时出现，位于输入位置上方 |
| 顶部提示 | `top_tip` / `TopTip` | 输入位置上方固定占用一行；提示文字右对齐，没有内容时整行留空，不与上方区域共享字符 |
| 输入位置 | `composer` / Composer Position | 页面为输入相关组件预留的位置；它是位置名称，不等于输入框 |
| 输入框 | `ChatInput` | 有上下边线、包含输入提示符和草稿文字的编辑框；这是输入位置的默认内容，具体字符见[样式契约](styles.md) |
| 状态行 | `StatusLine` | 输入位置下方的常态信息行，显示权限模式、模型、Git、Plan、Queue、Subagent 等摘要 |
| 操作提示行 | `KeyHints` | 某个交互需要明确操作时显示在输入位置下方两行的末行，例如 `↑↓ choose · enter confirm`；首行保持为空白分隔 |
| Agents | `agent_thread_switcher` / `AgentThreadSwitcher` | 页面最下方的 Agent Thread 列表；一行一个 `main` 或被委托 Agent，Enter 切换当前 Thread，只在存在被委托 Agent 时出现 |

### 正文单元会输出什么

正文区只读取一个按时间顺序排列的正文单元集合。执行单元在运行、接收输出和完成时始终是这个集合中的同一个元素；完成只更新内容和状态，不存在独立的实时输出集合或已完成历史集合。

| 内容类型 | 代码身份 | 对应内容 | 默认输出 | 展开或状态变化 |
| --- | --- | --- | --- | --- |
| 用户消息 | `MessageRole::User` | 已提交的用户文本和用户上下文 | `> 用户文本` | 多行内容继续与首行正文对齐 |
| Agent 消息 | `MessageRole::Agent` | Agent 的正文回复 | `● Agent 文本` | Markdown 和代码块在单元内折行与高亮 |
| 思考 | `MessageRole::Reasoning` | 可选展开的思考摘要 | `● Thought` | 展开后在下方输出有界摘要；过长内容可打开详情浮层 |
| 历史 Plan | `MessageRole::Plan` | 已进入正文的 Plan 文本或更新 | `● Plan 内容` | 它是正文记录，不是输入框上方的当前 `Plan` 行 |
| 执行 | `ExecCell` | Agent 发起的 Tool Call、命令、读取、搜索和修改 | `● Running <name>`、`● Ran <name>` 或 `● <name> failed` | 运行时原位更新；展开后用 `└─ ` 输出参数、`stdout`、`stderr` 和结果；多个同类操作可合并为一个单元 |
| 本地命令 | `ExecutionKind::LocalCommand` | 用户直接在 TUI 中提交的本地命令 | 运行时为 `● <command>`，结束后为 `> <command>` | 结果紧跟命令显示；它表达用户输入，不与 Agent 发起的执行归为同一类 |
| 提示 | `MessageRole::Notice` | 必须留在正文中的产品提示 | `● 提示文本` | 使用警告语义颜色，不伪装成 Agent 回复 |
| 错误 | `MessageRole::Error` | Turn 失败或无法归入执行单元的本地错误 | `● 错误摘要` | 可展开有界详情；过长内容可打开详情浮层；Tool Call 失败仍由 `ExecCell` 输出 |

下面的代码块只表示正文区里的可见字符，不表示颜色、背景色或当前选择状态。每个正文单元末尾都有一个空行；终端变窄时，正文会继续折行并与首行正文对齐，详情会与 `└─ ` 后的内容对齐。

#### 普通内容

单行用户消息、Agent 回复和连续对话分别输出为：

```text
> 帮我检查登录失败的问题

● 我会先检查错误路径和现有测试。
```

多行用户消息和多行 Agent 回复只有首行带角色标记：

```text
> 请检查这两个问题：
  登录失败后能否重试
  错误是否会留在正文中

● 登录失败会生成错误单元。
  下一轮仍然可以继续提交消息。
```

IDE、文件或其他用户上下文仍属于用户内容；第一行说明上下文名称，后续行输出内容：

```text
> Context · Active file
  zeta-code/tui/src/thread/transcript/exec.rs
```

当前图片和图片附件在正文中显示为占位文本：

```text
> [Image]
```

Agent 消息中的代码围栏不会作为字符显示；围栏内代码按语言高亮，普通文字仍按正文输出：

```text
● 可以把判断收敛到一个函数：
  fn is_complete(&self) -> bool {
      self.result.is_some()
  }
```

#### 思考、Plan、提示和错误

思考默认折叠，只显示固定摘要：

```text
● Thought
```

展开后在同一单元下显示思考内容：

```text
● Thought
└─ 先确认失败来自模型还是工具。
   再检查失败后能否开始下一轮。
```

思考超过 12 行时，正文只保留前 12 行和省略数量，并提供完整详情入口：

```text
● Thought
└─ 第 1 行
   第 2 行
   第 3 行
   第 4 行
   第 5 行
   第 6 行
   第 7 行
   第 8 行
   第 9 行
   第 10 行
   第 11 行
   第 12 行
   … 8 lines omitted
   view full
```

进入正文的 Plan 会输出说明和每一步的状态；`[ ]` 表示等待，`[>]` 表示正在进行，`[x]` 表示完成：

```text
● 先确认现状，再修改文档。
  [x] 核对正文单元类型
  [>] 补齐输出示例
  [ ] 检查文档链接
```

如果 Plan 没有说明也没有步骤，则输出：

```text
● Plan updated
```

提示和 Agent 回复都使用 `●`，但提示使用警告语义颜色：

```text
● Session 已切换，未提交的输入已恢复。
```

错误默认只显示第一行摘要，并使用失败语义颜色：

```text
● Model invocation failed
```

多行错误可以展开；超过 12 行时与思考使用相同的有界摘要和 `view full`：

```text
● Model invocation failed
└─ Model invocation failed
   HTTP 500 returned by the configured provider
   Request ID: request-123
```

错误不会结束整段正文。用户可以在它后面继续开始下一轮：

```text
> 触发 401 鉴权失败

● Model provider authentication failed

> 鉴权失败后继续下一轮

● 401 之后的下一轮恢复成功。
```

#### 用户本地命令

本地命令从提交到完成始终是同一个正文单元。下面三段表示同一位置先后出现的三个状态，不是正文中同时保留的三条记录。

刚提交、尚未开始时保留用户输入标记：

```text
> /theme zeta-code-dark
```

运行时原位变为警告语义的运行标记：

```text
● /theme zeta-code-dark
```

完成后原位恢复用户输入标记，并在下方输出结果：

```text
> /theme zeta-code-dark
└─ Theme set to Zeta Code Dark
```

打开命令面板、没有正文结果的本地命令只保留命令行；面板本身占用输入位置，不会伪装成命令详情：

```text
> /statusline
```

#### Agent 发起的单次执行

单次执行在折叠状态下只显示摘要。运行、成功和失败分别为：

```text
● Running write_file
```

```text
● Ran write_file
```

```text
● write_file failed
```

这三段也是同一个 `ExecCell` 的阶段变化。Tool Call 开始时插入单元，实时输出和最终结果继续更新这个单元，完成时摘要从 `Running` 原位变成 `Ran` 或 `failed`。

展开后，详情按工具名和 Tool Call 标识、参数、`stdout`、`stderr`、结果的顺序输出；没有内容的部分直接跳过：

```text
● Ran exec_command
└─ exec_command [call-test]
   {
     "cmd": "cargo test -p zeta-tui"
   }
   Compiling zeta-tui...
   warning: one retry was required
   42 tests passed
   view full
```

正文中的 `stdout`、`stderr` 和结果不额外显示字段标题，仍按上述固定顺序排列。命令输出中的 ANSI 颜色会转换为终端样式，ANSI 控制字符本身不会显示。

执行尚未完成但已经产生实时输出时，展开的是当前有界输出：

```text
● Running exec_command
└─ exec_command [call-test]
   {
     "cmd": "cargo test -p zeta-tui"
   }
   Compiling zeta-tui...
   running 42 tests
   view full
```

执行失败时仍可展开同一个单元查看参数和失败结果：

```text
● shell-command failed
└─ shell-command [call-sandbox]
   {
     "program": "/bin/sh",
     "arguments": ["-c", "touch ../outside.txt"]
   }
   operation not permitted
   view full
```

需要审批时，审批面板占用输入位置，不会进入正文集合。正文里的执行单元停留在运行态；用户批准后原位变成成功，用户拒绝、自动审查拒绝或沙盒阻止后原位变成失败：

| 阶段 | 正文区 | 其他区域 |
| --- | --- | --- |
| 等待用户审批 | `● Running write_file` | 输入位置显示审批面板和 `Approve once`、`Decline` |
| 用户批准并执行成功 | `● Ran write_file` | 审批面板关闭 |
| 用户或自动审查拒绝 | `● write_file failed` | 拒绝原因进入该执行单元的详情或后续错误说明 |
| 沙盒阻止命令 | `● shell-command failed` | 沙盒原因进入该执行单元的详情 |

#### 多次执行的合并

相邻的读取、搜索和列目录操作可以合并为一个探索单元。折叠时只显示操作数：

```text
● Explored 2 operations
```

展开后仍逐个保留工具名、Tool Call 标识和各自内容，并用空行分隔：

```text
● Explored 2 operations
└─ read_file [call-read]
   {
     "path": "zeta-code/docs/LAYOUT.md"
   }
   # Zeta Code TUI 界面词典与布局

   rg [call-search]
   {
     "pattern": "TranscriptCell"
   }
   zeta-code/docs/tui.md: TranscriptCell
   view full
```

相邻命令只有在前一个命令已经成功完成后才会合并。合并后折叠摘要为：

```text
● Ran 2 commands
```

展开后每个命令仍保持独立详情：

```text
● Ran 2 commands
└─ exec_command [call-check]
   {
     "cmd": "cargo check -p zeta-tui"
   }
   Finished dev profile

   exec_command [call-test]
   {
     "cmd": "cargo test -p zeta-tui"
   }
   42 tests passed
   view full
```

读取、搜索和列目录最多合并 16 次；命令组也最多合并 16 次。修改类工具和无法识别类别的工具不合并，每次 Tool Call 都保留自己的执行单元。探索组或命令组中只要仍有调用未完成，整个单元就使用运行语义颜色；全部完成后，只要有一个调用失败就使用失败语义颜色，否则使用成功或弱化语义颜色。首行始终保持 `Explored <数量> operations` 或 `Ran <数量> commands`。

#### 过长输出与完整详情

每个 `stdout` 或 `stderr` 实时输出最多保留 200 行、64 KiB；超过行数时保留头尾，并在完整详情中被裁剪的位置说明省略行数：

```text
… 36 lines omitted …
```

超过字节上限时同样保留头尾，并在中间显示：

```text
… output omitted …
```

实时输出和完成结果的单行都最多保留 4 KiB，过长行以 `…` 结尾。执行完成后的单个结果最多保留 256 KiB。

只要 `ExecCell` 保存了参数、输出或结果，展开后就会显示 `view full`，即使内联内容没有超过 12 行。内联预览最多显示合并详情的前 12 行；更多内容会先显示省略数量，再显示完整详情入口：

```text
● Ran exec_command
└─ exec_command [call-build]
   detail line 1
   detail line 2
   detail line 3
   detail line 4
   detail line 5
   detail line 6
   detail line 7
   detail line 8
   detail line 9
   detail line 10
   detail line 11
   … 28 lines omitted; view full
   view full
```

完整详情浮层不是新的正文单元，也不会复制执行记录。它覆盖在当前页面上，标题为 `Transcript cell`，显示该单元在上述容量限制内保存的详情，按 Esc 后回到原正文位置。思考和错误出现 `view full` 时也使用同一个浮层：

```text
Transcript cell
Content: exec_command [call-build]
         ...完整参数、输出和结果...
Esc to close
```

如果恢复 Session 时先收到输出或结果、没有收到对应的 Tool Call 开始记录，正文仍会恢复一个占位执行单元，折叠摘要使用 `Running tool`、`Ran tool` 或 `tool failed`，详情仍按 Tool Call 标识归入这个单元。

#### 一段完整正文

一次包含思考、执行失败、纠正和最终回复的正文会保持真实发生顺序：

```text
> 帮我运行测试并修复失败

● Thought

● exec_command failed
└─ exec_command [call-test-1]
   {
     "cmd": "cargo test -p zeta-tui"
   }
   test transcript::view failed
   view full

● 我找到失败原因了，会先修正文档断言。

● Ran apply_patch

● Ran exec_command
└─ exec_command [call-test-2]
   {
     "cmd": "cargo test -p zeta-tui"
   }
   42 tests passed
   view full

● 测试已经通过。
```

上例为了同时展示折叠与展开状态，手工让不同执行单元处于不同展开状态。任何一个执行单元从运行到完成都只更新原位置，不会把运行态另存为一条“历史单元”。

#### 颜色和交互补充

| 情况 | 可见标记 | 语义颜色或背景 |
| --- | --- | --- |
| 用户消息、本地命令输入 | `>` | 弱化标记；输入正文行使用用户消息背景 |
| Agent 消息、思考、Plan | `●` | 弱化标记 |
| 提示 | `●` | 警告色 |
| 错误、失败执行 | `●` | 失败色 |
| 运行中的执行或本地命令 | `●` | 警告色 |
| 成功的命令执行 | `●` | 成功色 |
| 成功的修改执行 | `●` | 强调色 |
| 成功的读取、搜索、列目录或其他执行 | `●` | 弱化色 |

键盘选中可展开单元后按 Space 切换展开状态，鼠标可以点击首行左侧标记完成同一操作。键盘选中已展开单元后按 Enter 打开完整详情，鼠标也可以点击 `view full`。键盘选择、悬停和按下只改变交互样式，不改变正文字符、单元身份或内容顺序。

内部类型如何产生这些行、谁负责测量、缓存和命中，由 [TUI 架构的 Transcript 章节](tui.md#transcript) 定义。

### 输入位置里可能出现什么

“Composer”最容易被误当成输入框。准确说法是：**输入位置是槽位，输入框只是默认放进去的组件。**

代码中的 `ChatPanel` 负责 Session 页面底部整块聊天交互区：固定内容包括 `TopTip` 和
`StatusLine`，按状态出现的内容包括 `ChatInput`、`Approval`、`Query` 和 `CommandPanel`。
“固定”和“临时”只描述显示周期，不再各自形成一种容器类型；页面上的具体内容仍使用下表名称。

本地 Slash Command 打开并替换输入框的界面统一称为**命令面板**，具体面板使用标题加“面板”，例如 Session 面板、Model 面板、Theme 面板、Config 面板和 Status 面板。是否包含候选列表、搜索或编辑能力不再产生新的产品术语。

`CommandPanel` 是 `ChatPanel` 内当前命令面板的代码容器；`Picker`、`Editor` 和 `Panel` 等后缀只表达代码内部职责，不作为用户描述界面时的分类。输入 `/` 时出现的候选尚未执行命令，应称为 Slash Command 补全浮层，不叫命令面板。

| 当前内容 | 推荐叫法 | 与其他区域的关系 |
| --- | --- | --- |
| 普通草稿输入 | 输入框（`ChatInput`） | 默认占用输入位置 |
| 权限确认 | 审批面板（`Approval`） | 替换输入框，占用输入位置 |
| `/status`、`/theme`、`/model`、`/config`、`/resume` 等本地命令界面 | 命令面板；需要具体定位时使用标题加“面板” | 替换输入框，占用输入位置；内部可以显示信息、候选列表、搜索或编辑内容 |
| Agent 的结构化问题 | 提问面板（`Query`） | 不替换输入位置，而是在它上方单独占区；需要自由输入时仍使用下方输入框 |

### 输入框附近的两种提示

| 推荐名称 | 位置 | 用途 |
| --- | --- | --- |
| 顶部提示 | `TopTip` | 在输入位置上方固定占用一整行，文字靠右；剪贴板有图片时显示 `image in clipboard · ctrl+v to paste`，否则按状态显示空会话导航或限时权限提示；临时通知优先于剪贴板图片提示，所有内容切换都不改变这一行的高度 |
| 操作提示行（HitBar） | `KeyHints` | 位于输入位置下方两行的末行；交互时替换状态行的末行，首行保持为空白分隔，底部两行总高度不变 |

因此，剪贴板图片提示、`← for agents` 和 `shift+tab to cycle policy` 都属于顶部提示；输入框下方的权限模式、模型和 Git 信息才属于状态行。这里没有轮播：权限提示由首次进入对话或切换权限策略触发；连续切换会刷新计时，距最后一次触发 5 秒后直接消失。剪贴板图片提示由启动和窗口重新获得焦点时的检测结果决定。

## 4. Session 管理页面

Session 管理页面复用正文区的位置，但内部改为 Welcome 和 Session 列表：

```text
┌──────────────────────────────────────────────────────────┐
│ Welcome                                                  │
│                                                          │ ← 有空间时保留 1 行
│ Session 列表 Session Manager                             │
│   ├─ 状态分组标题                                        │
│   └─ Session 行：图标 / 名称 / 当前操作或问题 / 时长      │
│ 顶部提示 TopTip（固定一行，提示为空时留空）              │
├──────────────────────────────────────────────────────────┤
│ 输入位置：ChatInput 或命令面板                           │
├──────────────────────────────────────────────────────────┤
│ 空白分隔行                                               │
│ 操作提示栏 HitBar（KeyHints）                           │
└──────────────────────────────────────────────────────────┘
```

Session 列表内部按以下横向结构排列：

```text
Idle (2)                                  ← 静态分组标题
> ○ Session 名称     当前操作或问题    2h ← 键盘当前 Session
  ○ 另一 Session                         ← 普通 Session
└─┘└┘└──────────────┘└──────────────┘└──┘
 标 状态     名称          活动          时长
 记 图标
 栏
```

| 横向部位 | 宽度与位置 | 内容与边界 |
| --- | --- | --- |
| 分组标题 | 独占整行，从最左侧开始 | 显示状态组名称和 Session 数量；它是静态标签，不接受键盘或鼠标操作，也不能折叠 |
| 标记栏 | Session 行最左侧 2 格 | 管理页面获得焦点时，键盘当前 Session 显示 `> `；其他 Session 保留两个空格 |
| 状态图标 | 标记栏之后 1 格 | 表达空闲、工作、等待输入、待审查、完成、失败或停止；它不表示键盘当前项 |
| 名称 | 状态图标后空 1 格开始 | Session 名称；空间不足时截断 |
| 活动 | 名称右侧 | 当前操作、问题、失败摘要或 Session 摘要；窄宽度下优先隐藏 |
| 时长 | 最右侧 | 工作时长或完成距今时间；不足 1 小时时可以为空 |

分组顺序固定为 Pinned、Needs input、Working、Ready for review、Failed、Stopped、Completed、Idle；空分组不显示。上下键只在 Session 行之间移动，会跳过分组标题。Space 预览当前 Session，Enter 恢复当前 Session，`Ctrl+X` 只归档当前 Session。

| 推荐名称 | 代码名称 | 内容与边界 |
| --- | --- | --- |
| Welcome | `welcome` | 顶部欢迎文字和当前工作区；终端过矮时先缩短这部分 |
| Session 列表 | `sessions` / `SessionManager` | 按状态分组的 Session 行，不要称为 Transcript |
| Session 行 | Session row | 列表中的单个 Session；定位时最好同时说出分组名和 Session 名 |

## 5. 浮层

浮层画在普通页面之上，不占用上面那些区域的高度。

```text
最后绘制，位于最上层
  字符选择 Screen Selection
  详情浮层 DetailOverlay       ┐ 同一帧二选一
  输入补全浮层 CompletionView  ┘
普通 Session 页面或 Session 管理页面
```

| 推荐名称 | 代码名称 | 怎么认 |
| --- | --- | --- |
| 详情浮层 | `DetailOverlay` | 靠可用区域底部、背景和内容都铺满整行的只读详情层，标题、详情和关闭提示遵守页面内容缩进，通常有 `Esc to close`；打开时阻止底层操作 |
| 输入补全浮层 | `CompletionView` | 输入 `/`、`@`、`$` 时出现在输入框上方的候选列表；有详情浮层时不显示 |
| Slash Command 补全 | `CompletionView::Slash` | 输入 `/` 时显示产品或 App Server 提供的 Slash Command 候选 |
| Mention 补全 | `CompletionView::Mention` | 输入 `@` 时显示文件或 Plugin 上下文候选 |
| Skill 补全 | `CompletionView::Skill` | 输入 `$` 时显示可调用 Skill 候选 |
| 字符选择 | `ScreenSelection` | 鼠标拖选、双击或三击形成的终端字符选择效果，画在所有内容之上 |

`/status` 打开的是输入位置中的 Status 面板（代码类型为 `StatusPanel`），不是详情浮层。它依次显示模型、上下文窗口、当前 Thread
的累计模型调用数、输入 token、缓存读取 token、缓存读取占比、缓存写入 token、输出 token、
推理输出 token、累计参考费用，以及 Session/Thread 身份。空间足够时它按完整内容展开；空间不足时
至少保留 4 行正文，并用方向键、PageUp/PageDown、Home/End 滚动。缓存占比是
`缓存读取 token / 总输入 token`，不是请求次数的命中率。聚合缺少部分报告时，已知非零 token
以 `>=` 标记为下界，已知费用以 `≥` 标记为下界；没有可信值时显示 `unknown`。

不要把审批面板、提问面板叫“弹窗”：它们会参与普通纵向布局。只有覆盖在页面之上的
`DetailOverlay` 和输入补全浮层才叫浮层。三种补全是同一个输入补全浮层的互斥内容，不是三个不同浮层；描述具体问题时使用“Slash Command 补全浮层”“Mention 补全浮层”或“Skill 补全浮层”。

## 6. 高度变化时会发生什么

- 没有内容的 Goal、Plan、Queue、Query 和 Agents 高度为 0，不留下空壳。
- 正文区拿走普通组件分配后剩余的高度；空间允许时至少保留 4 行。
- 顶部提示固定占用一行；没有提示文字时这一行保持为空。
- 输入框高度随多行草稿变化；审批面板和命令面板会用自己的高度替换它。
- 输入位置下方固定占用两行。常态显示 `StatusLine`；命令面板或其他需要明确操作的交互打开时，首行留空，末行显示操作提示。
- 底部两行和 Agents 都出现时，中间保留一行；只出现一方时不保留。
- Session 管理页面至少给 Session 列表保留 4 行；终端变矮时先压缩 Welcome。

因此，定位“组件消失”问题时必须附上终端宽高。`80×24` 表示 80 列、24 行。

## 7. 报问题时这样描述

推荐使用以下顺序：

```text
页面 / 区域或浮层 / 具体元素 / 当前状态 / 终端尺寸 / 实际现象
```

例如：

- `Session 页面 / StatusLine / 左侧权限模式 / 80×24 / 和模型名称挤在一起`
- `Session 页面 / ChatInput / 第二行草稿 / Vim Normal / 120×30 / 光标位置偏右一格`
- `Session 页面 / Transcript / 标记栏 / Agent 正文单元 / 80×24 / 圆点与正文之间多出一格`
- `Session 页面 / Transcript / 某个工具正文单元 / 展开状态 / PageUp 后内容跳动`
- `Session 页面 / Query / 第三个选项 / 60×20 / 鼠标点到下一项`
- `Session 管理页面 / Working 分组 / foo Session 行 / 100×28 / 时长列被截断`
- `Session 页面 / Mention 补全浮层 / @ 文件候选 / 80×24 / 第一项盖住输入框上边线`
- `Session 页面 / Slash Command 补全浮层 / /skills 候选项 / 80×24 / 第一项盖住输入框上边线`
- `Session 页面 / 输入位置 / Status 面板 / 80×18 / Thread ID 被截断`

如果仍不知道名称，可以说“在 A 和 B 之间的那一行”，但 A、B 尽量使用本文术语。
