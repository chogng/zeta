# `zeta-tui` 布局

> 本文只说明当前界面区域。完整设计依据见 [`docs/tui-chat-architecture-discussion-v15.md`](../../../docs/tui-chat-architecture-discussion-v15.md)。

## 快速理解

Session 页面由 [`app/screen_layout.rs`](app/screen_layout.rs) 唯一分配整页几何：

```text
Transcript                         弹性剩余空间
Goal                               0..1 行
Plan                               0..1 行
Queue                              0..3 行
Query                              0..1 行，位于 ChatInput 上方
ChatInput 或 Approval              同一输入区域二选一
StatusLine 及补充 KeyHints       1..3 行；操作 KeyHints 固定 1 行
SubagentPane                       0..4 行
```

Query 使用自己的选择和文本编辑状态，普通输入框保持可见且草稿不变。Approval 替换普通输入区域，结束后原草稿原样恢复。Goal、Plan、Queue 不进入上下方向的焦点链。

StatusLine 持续显示系统状态。当空输入下存在可用的根页面导航时，补充 KeyHints 与 StatusLine 同行右对齐，宽度不足时独占一行。正文单元进入选择态、SubagentPane 或其他需要明确操作说明的表面获得焦点时，操作 KeyHints 暂时接管该区域。

## Session Manager

```text
Manager body/list
ChatInput
StatusLine 及补充 KeyHints，或操作 KeyHints
```

Manager 是横向根页面的最左端：`Manager ← Session 1 ← Session 2 → Session 3`。空输入时 Left/Right 切换根页面；非空时仍由编辑器移动光标。

## 覆盖层和页面

- QuickView 覆盖现有内容，不改变正常布局高度，并独立保存滚动位置；正文详情仍由稳定 `TranscriptCellId` 打开。
- stacked Pane 从 ChatInput 向上占高，只显示 `PaneStack` 栈顶；`/queue` 等管理页面使用 Pane。
- ChatInput 补全弹层锚定输入框上沿覆盖；Query 和 Approval 使用上面的独立布局区域。
- SubagentPane 常驻于 StatusLine/KeyHints 下方，只显示 Main 与活动 Subagent，最多四行。

绘制与鼠标命中复用同一份区域结果。组件只解释局部输入；Session、Thread、Queue、Goal、Plan、Approval 和 Query 的含义留在对应 feature。
