# 正文输出与阅读

- 本文规定用户消息、Agent 回复、执行结果、合并输出和完整详情的可见结构。
- 页面空间见[布局](layout.md)，字符与颜色见[正文样式](styles.md#正文样式)，类型与绘制一致性见[正文绘制约定](transcript-rendering.md)。
- 示例用于说明输出要求；实际支持和验证限制见[功能现状](../capabilities.md)。

## 阅读入口

| 要查什么 | 章节 |
| --- | --- |
| 每类内容的身份和输出 | [单元类型](#正文单元会输出什么) |
| 消息、代码、图片占位 | [普通内容](#普通内容) |
| 思考、Plan、提示和错误 | [状态内容](#思考plan提示和错误) |
| 用户命令 | [本地命令](#用户本地命令) |
| 工具参数、实时输出和结果 | [单次执行](#agent-发起的单次执行) |
| 多个调用怎样合并 | [合并](#多次执行的合并) |
| 容量限制和完整详情 | [过长输出](#过长输出与完整详情) |

## 阅读、复制与导出

| 场景 | 要求 |
| --- | --- |
| 阅读长输出 | 宽窄窗口中可以折行阅读，保留截断说明；上翻后新输出不强制跳到底部 |
| 条目选择 | 空输入时 Ctrl+↑ / Ctrl+↓ 进入；方向键或 j/k、Home/End、PageUp/PageDown 导航，每次翻页 12 条 |
| 展开和详情 | Space 展开，Enter 查看完整详情；Esc 返回输入或原正文位置 |
| 回到底部 | 使用实际可用的 Ctrl+End；正文不属于增强鼠标捕获范围 |
| 复制 | 应用快捷键上下文中 Ctrl+O 复制最后回复 |
| 导出 | 导出当前已加载正文到本机工作目录；不覆盖已有文件，拒绝越界路径；失败不改变任务状态 |
| 终端历史 | 已定稿内容提交、分页恢复和终端回滚见[终端规则](terminal.md#鼠标规则) |


以下示例用于核对单条消息和执行输出的内部结构；整页空间分配以正文规则为准。

## 正文单元会输出什么

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

## 普通内容

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

## 思考、Plan、提示和错误

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

## 用户本地命令

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

## Agent 发起的单次执行

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

## 多次执行的合并

相邻的读取、搜索和列目录操作可以合并为一个探索单元。折叠时只显示操作数：

```text
● Explored 2 operations
```

展开后仍逐个保留工具名、Tool Call 标识和各自内容，并用空行分隔：

```text
● Explored 2 operations
└─ read_file [call-read]
   {
     "path": "zeta-code/docs/spec/layout.md"
   }
   # Zeta Code TUI 界面词典与布局

   rg [call-search]
   {
     "pattern": "TranscriptCell"
   }
   zeta-code/docs/design/tui.md: TranscriptCell
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

- 读取、搜索和列目录最多合并 16 次；命令组也最多合并 16 次。
- 修改类工具和无法识别类别的工具不合并，每次 Tool Call 都保留自己的执行单元。
- 探索组或命令组中只要仍有调用未完成，整个单元就使用运行语义颜色；全部完成后，只要有一个调用失败就使用失败语义颜色，否则使用成功或弱化语义颜色。
- 首行始终保持 `Explored <数量> operations` 或 `Ran <数量> commands`。

## 过长输出与完整详情

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

- 完整详情浮层不是新的正文单元，也不会复制执行记录。
- 它覆盖在当前页面上，标题为 `Transcript cell`，显示该单元在上述容量限制内保存的详情，按 Esc 后回到原正文位置。
- 思考和错误出现 `view full` 时也使用同一个浮层：

```text
Transcript cell
Content: exec_command [call-build]
         ...完整参数、输出和结果...
Esc to close
```

如果恢复 Session 时先收到输出或结果、没有收到对应的 Tool Call 开始记录，正文仍会恢复一个占位执行单元，折叠摘要使用 `Running tool`、`Ran tool` 或 `tool failed`，详情仍按 Tool Call 标识归入这个单元。

## 一段完整正文

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

## 颜色和交互补充

- 以上示例只解释可见行和内容归属。
- 字符与颜色映射统一见[正文样式](styles.md#正文样式)，展开和完整详情的键盘动作见[交互键表](conversation.md#页面辅助区域与浮层)。
- 正文不属于当前增强鼠标捕获范围，不能从示例中的动作文字推断它具有鼠标点击入口。

内部类型如何产生这些行、谁负责测量、缓存和命中，由 [TUI 架构的 Transcript 章节](../design/tui.md#正文与终端历史) 定义。
