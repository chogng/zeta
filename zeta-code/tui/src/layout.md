# `zeta-tui` 布局

> 本文只说明当前界面区域。空间语义、owner 和生命周期见 [`zeta-code/docs/tui.md`](../../docs/tui.md#131-界面结构输入位置与覆盖层)。

## 快速理解

Session 页面由 [`app/screen_layout.rs`](app/screen_layout.rs) 唯一分配整页几何：

```text
Transcript                         弹性剩余空间
Goal                               0..1 行
Plan                               0..1 行
Queue                              0..3 行
Query                              0..1 行，位于 ChatInput 上方
ChatInput、Approval 或 Composer 当前内容  共用输入位置，只显示一个
StatusLine 及补充 KeyHints       1..3 行；操作 KeyHints 固定 1 行
SubagentPicker                    0..4 行
```

Query 使用自己的选择和文本编辑状态，普通输入框保持可见且草稿不变。Approval 替换普通输入区域，结束后原草稿原样恢复。Goal、Plan、Queue 不进入上下方向的焦点链。

StatusLine 持续显示系统状态。当空输入下存在可用的界面导航时，补充 KeyHints 与 StatusLine 同行右对齐，宽度不足时独占一行。正文单元进入选择态、SubagentPicker 或 Composer 当前内容获得焦点时，操作 KeyHints 暂时接管底栏。

## Session Manager

```text
Manager body/list
ChatInput
StatusLine 及补充 KeyHints，或操作 KeyHints
```

Manager 和当前 Session 是两个 `TerminalScreen`：`Manager ←→ Session`。空输入时 Left/Right 切换界面；非空时仍由编辑器移动光标。历史 Session 通过 Manager 选择，不是额外的横向界面。

## 输入位置和覆盖层

- App 同时最多保存一个 `ComposerMode`。它记录 Session 输入位置当前显示的具体组件；该组件替换 ChatInput、报告自己的高度和 KeyHints。多步返回关系由 Config、Keymap、Theme 等 feature 自己保存。
- App 同时最多保存一个 `DetailOverlay`。Overlay 覆盖当前帧、不进入 `screen_layout`，并阻止底层键盘和鼠标输入；Status、Session preview、正文详情和 Queue 详情共用该入口。
- ChatInput completion 锚定输入框上沿覆盖，由 ChatInput 保存。同一帧优先画应用级 Overlay，否则才画 completion。
- SubagentPicker 常驻于 StatusLine/KeyHints 下方，只显示 Main 与活动 Subagent，最多四行。

绘制顺序是 `TerminalScreen 的普通组件 → Overlay/completion → 字符框选`，鼠标命中按相反顺序执行。“占多少行”只进入 `screen_layout`，不对应组件类型。组件只解释局部输入；Session、Thread、Queue、Goal、Plan、Approval 和 Query 的含义留在对应 feature。
