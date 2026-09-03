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

## 2. Session 页面

下面是各区域都出现时的纵向顺序。实际界面会隐藏没有内容的区域，所以它们不一定同时可见。

```text
┌──────────────────────────────────────────────────────────┐
│ 正文区 Transcript                                        │
│   ├─ Welcome（空 Session 时）                            │
│   └─ 正文单元 Transcript Cell                            │
│                                                          │
├──────────────── Goal 行（有目标时）─────────────────────┤
├──────────────── Plan 行（有进行中计划时）───────────────┤
├──────────────── Queue 区（有排队消息时，最多显示 3 条）─┤
│ Query 面板（Agent 提问时，位于输入位置上方）             │
├──────────────────────────────────────────────────────────┤
│ 输入位置 Composer Position                               │
│   通常显示 ChatInput；也可能被 Approval 或功能页面替换    │
├──────────────────────────────────────────────────────────┤
│ 状态区 Status Area                                       │
│   通常显示 StatusLine；需要操作时改为 KeyHints            │
│                                                          │ ← 条件性间隔
│ Agents AgentThreadSwitcher（有被委托 Agent 时）          │
└──────────────────────────────────────────────────────────┘
```

### 固定区域词典

| 推荐名称 | 代码名称 | 内容与边界 |
| --- | --- | --- |
| 正文区 | `transcript` / `ChatHistory` | 对话内容占据的主要滚动区域；空 Session 的 Welcome 也画在这里 |
| 正文单元 | `TranscriptCell` | 正文区内的一条用户消息、Agent 回复、命令或工具结果；“某条消息”要定位到这一层 |
| Goal 行 | `goal` | 一行 `Goal: …`；无目标时高度为 0 |
| Plan 行 | `plan` | 一行 `Plan 已完成数/总数: 当前步骤`；计划结束后隐藏 |
| Queue 区 | `queue` | 输入在当前 Turn 结束后再发送时，显示排队消息；普通输入框不属于这里 |
| 提问面板 | `request` / `Query` | Agent 要求用户选择或输入答案时出现，位于输入位置上方 |
| 输入位置 | `composer` / Composer Position | 页面为输入相关组件预留的位置；它是位置名称，不等于输入框 |
| 输入框 | `ChatInput` | 有上下边线、包含输入提示符和草稿文字的编辑框；这是输入位置的默认内容，具体字符见[样式契约](styles.md) |
| 状态区 | `status` / Status Area | 输入位置下面的一至两行槽位；在状态行和按键提示之间切换 |
| 状态行 | `StatusLine` | 正常状态下显示权限模式、模型、Git、Plan、Queue、Subagent 等摘要 |
| 按键提示 | `KeyHints` | 某个交互需要明确操作时替换状态行，例如 `↑↓ choose · enter confirm` |
| Agents | `agent_thread_switcher` / `AgentThreadSwitcher` | 页面最下方的 Agent Thread 列表；一行一个 `main` 或被委托 Agent，Enter 切换当前 Thread，只在存在被委托 Agent 时出现 |

### 输入位置里可能出现什么

“Composer”最容易被误当成输入框。准确说法是：**输入位置是槽位，输入框只是默认放进去的组件。**

| 当前内容 | 推荐叫法 | 与其他区域的关系 |
| --- | --- | --- |
| 普通草稿输入 | 输入框（`ChatInput`） | 默认占用输入位置 |
| 权限确认 | 审批面板（`Approval`） | 替换输入框，占用输入位置 |
| Theme、Model、Config、Session picker 等页面 | 功能页面（`ComposerMode`） | 替换输入框，占用输入位置；描述问题时再补具体页面名 |
| Agent 的结构化问题 | 提问面板（`Query`） | 不替换输入位置，而是在它上方单独占区；需要自由输入时仍使用下方输入框 |

### 输入框附近的两种提示

| 推荐名称 | 位置 | 用途 |
| --- | --- | --- |
| 顶部提示 | `TopTip` | 固定贴在输入框上沿右侧；空会话默认显示 `← for agents`，首次提交进入对话后改为 `shift+tab to cycle policy`，保留 5 秒后留空；临时通知会短暂覆盖它，不改变布局高度 |
| 状态区按键提示 | `KeyHints` | 位于输入位置下方，会替换 `StatusLine`，并参与布局高度计算 |

因此，`← for agents` 和一次性的 `shift+tab to cycle policy` 都属于顶部提示；输入框下方的权限模式、模型和 Git 信息才属于状态行。这里没有轮播：提示只会在进入对话时切换一次，到期后直接消失。

## 3. Session 管理页面

Session 管理页面复用正文区的位置，但内部改为 Welcome 和 Session 列表：

```text
┌──────────────────────────────────────────────────────────┐
│ Welcome                                                  │
│                                                          │ ← 有空间时保留 1 行
│ Session 列表 Session Manager                             │
│   ├─ 状态分组标题                                        │
│   └─ Session 行：图标 / 名称 / 当前操作或问题 / 时长      │
├──────────────────────────────────────────────────────────┤
│ 输入位置：ChatInput 或功能页面                           │
├──────────────────────────────────────────────────────────┤
│ 按键提示 KeyHints                                        │
└──────────────────────────────────────────────────────────┘
```

| 推荐名称 | 代码名称 | 内容与边界 |
| --- | --- | --- |
| Welcome | `welcome` | 顶部欢迎文字和当前工作区；终端过矮时先缩短这部分 |
| Session 列表 | `sessions` / `SessionManager` | 按状态分组的 Session 行，不要称为 Transcript |
| Session 行 | Session row | 列表中的单个 Session；定位时最好同时说出分组名和 Session 名 |

## 4. 浮层

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
| 详情浮层 | `DetailOverlay` | 居中、靠可用区域底部的只读详情框，通常有 `Esc to close`；打开时阻止底层操作 |
| 输入补全浮层 | `CompletionView` | 输入 `/`、`@`、`$` 时出现在输入框上方的候选列表；有详情浮层时不显示 |
| Slash Command 补全 | `CompletionView::Slash` | 输入 `/` 时显示产品或 App Server 提供的 Slash Command 候选 |
| Mention 补全 | `CompletionView::Mention` | 输入 `@` 时显示文件或 Plugin 上下文候选 |
| Skill 补全 | `CompletionView::Skill` | 输入 `$` 时显示可调用 Skill 候选 |
| 字符选择 | `ScreenSelection` | 鼠标拖选、双击或三击形成的终端字符选择效果，画在所有内容之上 |

不要把审批面板、提问面板叫“弹窗”：它们会参与普通纵向布局。只有覆盖在页面之上的
`DetailOverlay` 和输入补全浮层才叫浮层。三种补全是同一个输入补全浮层的互斥内容，不是三个不同浮层；描述具体问题时使用“Slash Command 补全浮层”“Mention 补全浮层”或“Skill 补全浮层”。

## 5. 高度变化时会发生什么

- 没有内容的 Goal、Plan、Queue、Query 和 Agents 高度为 0，不留下空壳。
- 正文区拿走普通组件分配后剩余的高度；空间允许时至少保留 4 行。
- 输入框高度随多行草稿变化；审批面板和功能页面会用自己的高度替换它。
- 状态区通常最多显示两行，但 `KeyHints` 可以临时替换状态行。
- 状态区和 Agents 都出现时，中间保留一行；只出现一方时不保留。
- Session 管理页面至少给 Session 列表保留 4 行；终端变矮时先压缩 Welcome。

因此，定位“组件消失”问题时必须附上终端宽高。`80×24` 表示 80 列、24 行。

## 6. 报问题时这样描述

推荐使用以下顺序：

```text
页面 / 区域或浮层 / 具体元素 / 当前状态 / 终端尺寸 / 实际现象
```

例如：

- `Session 页面 / StatusLine / 左侧权限模式 / 80×24 / 和模型名称挤在一起`
- `Session 页面 / ChatInput / 第二行草稿 / Vim Normal / 120×30 / 光标位置偏右一格`
- `Session 页面 / Transcript / 某个工具正文单元 / 展开状态 / PageUp 后内容跳动`
- `Session 页面 / Query / 第三个选项 / 60×20 / 鼠标点到下一项`
- `Session 管理页面 / Working 分组 / foo Session 行 / 100×28 / 时长列被截断`
- `Session 页面 / Mention 补全浮层 / @ 文件候选 / 80×24 / 第一项盖住输入框上边线`
- `Session 页面 / Slash Command 补全浮层 / /skills 候选项 / 80×24 / 第一项盖住输入框上边线`
- `Session 页面 / DetailOverlay / /status 详情 / 80×18 / 底部按键提示不可见`

如果仍不知道名称，可以说“在 A 和 B 之间的那一行”，但 A、B 尽量使用本文术语。
