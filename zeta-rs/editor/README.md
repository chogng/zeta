# `zeta-editor`

> 本 README 是代码编辑器与差异编辑器 presentation 的 crate-level canonical contract。
> 跨 crate 的依赖与产品宿主边界见
> [`docs/zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md)；`zeta-diff` 算法契约见
> [`diff/README.md`](../diff/README.md)；异步结构分析与 revision binding 见
> [`docs/syntax-analysis.md`](../../docs/syntax-analysis.md)。

`zeta-editor` 拥有 Native UI 使用的多行代码编辑模型、caret/selection、键盘命令、有界
undo/redo、IME composition、语法 token、代码行与视口绘制，以及由两个代码视口组成的并排差异
展示、适合窄 surface 的单列 unified 差异展示，以及纵向组合多个文件差异的
`MultiDiffEditor`。它不读取文件、不执行 Git、不计算 diff，也不拥有 changed-file collection、
活动 EditorHost 或平台事件。

## 所有权与接口

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `CodeEditor` | public | 绘制可见代码行、caret/selection、preedit、syntax token、gutter 与 decoration；`within_viewport` 限制嵌入式宿主实际投影的行 |
| `CodeEditorPresentation` | public | 选择带 document chrome 的普通编辑器或隐藏 gutter 的 compact 嵌入式编辑器 |
| `CodeEditorDocument` | public | 保存文本、行 range、selection、composition、syntax snapshot 与 undo/redo |
| `CodeEditorCommand` | public | 表达插入、换行、Unicode navigation、选择、删除与 undo/redo |
| `CodeEditorSyntaxHighlighter` | public trait | 把单行文本同步投影为 UTF-8 byte-range foreground token |
| `CodeEditorRowSource` | public trait | 惰性提供稳定 visual row；普通文档和 diff projection 共用 |
| `CodeEditorRow` | public | 表达真实代码行、对齐 placeholder 或无行号 annotation，以及本帧 decoration |
| `CodeEditorViewport` | public | 保存首个可见行和横向显示列，并执行有界滚动或 reveal-row |
| `CodeEditorStyle` | public | 拥有代码 surface、header、gutter 与文本的浅色 presentation token |
| `DiffEditor` | public | 按 presentation 组合双列或单列 `CodeEditor`，同步纵向 viewport 并绘制 diff decoration |
| `DiffEditorPresentation` | public | 显式选择 `SideBySide` 或适合窄嵌入 surface 的 `Unified` geometry |
| `DiffEditorState` | public | 保存共享首行、两侧独立横向显示列和 Unified 未修改区间的展开状态 |
| `DiffEditorFoldControl` | public | 向产品宿主发布可见未修改区间的行数、状态与命中 bounds |
| `MultiDiffEditor` | public | 把多个文件标题和 `DiffEditor` section 组合为一个纵向裁剪 surface |
| `MultiDiffEditorItem` | public | 为一帧借用文件名、`DiffDocument`、两侧标签和该文件的 `DiffEditorState` |
| `MultiDiffEditorLayout` | public | 缓存一个精确 item/state/presentation snapshot 的 section heights 与总内容高度，供高频滚动复用 |
| `zeta-ui::ScrollState` | delegated | 保存 MultiDiffEditor 整体 logical-pixel offset；clamp 与 transition 由通用滚动基座执行 |
| `MultiDiffEditorStyle` | public | 拥有文件 header、section 间距与嵌套 DiffEditor 样式 |
| `DiffSideRows` | private | 把 `DiffDocument` 的一侧惰性转换为 `CodeEditorRow` |
| `UnifiedDiffRows` | private | 用 hunk source-range、修改行索引和 fold segment 紧凑表达单列 diff；按可见 visual index 随机映射代码行，不按文件总行数分配 row 数组 |
| `code_editor::layout::build_layout` | private | 从组件 bounds 计算 header/body/gutter/content |
| `code_editor::text_metrics::display_columns_until` | private | 把 UTF-8 byte offset 映射到 Tab/Unicode 等宽显示列 |
| `code_editor::editing` | private module | 执行 grapheme-safe mutation、跨行导航、selection 与有界 history |
| `code_editor::decorations` | private module | 绘制 selection、syntax、composition、caret 并计算 IME caret bounds |
| `diff_editor::layout::build_layout` | private | 计算两个 pane 与中央 divider |

```text
CodeEditorDocument ─┐
                    ├─ CodeEditorRowSource::row
DiffSideRows ───────┘
                           ↓
CodeEditor::paint
├─ code_editor::layout::build_layout
├─ CodeEditor::visible_row_range
├─ CodeEditor::painted_row_range → host viewport row culling
├─ gutter / marker / line background
├─ text_metrics::display_columns_until
└─ UiScene

CodeEditorDocument::apply
├─ CodeEditorCommand
├─ grapheme / CRLF boundary navigation
├─ selection replacement
├─ checkpoint → bounded undo / redo
└─ reindex_lines → clear stale syntax

CodeEditorDocument::apply_composition
├─ Preedit → separate composition state
├─ Commit → one undoable replacement
└─ Cancel → committed text unchanged

DiffEditor::paint
├─ SideBySide
│  ├─ DiffSideRows(original) → CodeEditor
│  ├─ DiffSideRows(modified) → CodeEditor
│  └─ divider
└─ Unified
   └─ UnifiedDiffRows → one CodeEditor
      ├─ context → one row
      └─ modified → removed row + added row
      └─ long unchanged range → fold control + optional source rows

MultiDiffEditor::paint
├─ ScrollView::draw
│  ├─ viewport clip / content origin
│  └─ hover/active/fading scrollbar chrome
└─ visible MultiDiffEditorItem
   ├─ file header
   └─ DiffEditor → selected SideBySide or Unified presentation
```

`CodeEditorRowSource` 实现必须在一个 presentation frame 内保持 row ordering 和 line-number
identity 稳定，并只为请求的可见 index 构造行。普通文档返回真实编号行；差异投影可以返回没有
line number 的 placeholder 或 fold annotation，但不能把它们写入 authoritative document。行内高亮使用
UTF-8 byte range；越界或不落在字符边界上的 range 会被忽略，不得造成绘制 panic。
Code row 使用 `TextBlockWrap::None`，并把宽字符 grapheme 与 whitespace 按 8px display cell
边界分别投影；ASCII run 对齐一个 cell，CJK fallback 对齐两个 cell，使 text、space、
selection 与 caret 共用同一坐标系。

Unified projection 以 `DiffDocument::hunks` 的间隙作为可折叠区间，只保存少量 source segment
和 `Modified` source-row index。展开超大未修改区间不会物化等量 `CodeEditorRow`；可见行通过
二分映射回 source row。`MultiDiffEditor` 在一个 presentation 实例内缓存每文件 section height；
高频输入宿主可继续保留 `measure_layout` 结果，使 scroll metrics、fold-control geometry 和 paint
跨 presentation rebuild 共用同一组高度。item、document、`DiffEditorState`、style 或 presentation
变化时必须重新测量。

## 执行、失败与宿主义务

`CodeEditor`、`DiffEditor` 和 `MultiDiffEditor` 构造与绘制没有 I/O 或独立 error channel。编辑命令以 Unicode
grapheme boundary 修改 committed text；CRLF 在删除时作为一个换行边界处理。history 最多保留
100 个完整 snapshot，新编辑会清空 redo；当前不合并连续输入。IME preedit 与 committed text
分离，只有 Commit 建立一个可撤销 checkpoint，Cancel 不修改文档。`CodeEditor::caret_bounds`
向平台宿主提供候选框锚点，`text_position_at` 与 `CodeEditorDocument::move_to` /
`set_selection` 支持指针选择。`CodeEditorPresentation::Compact` 只改变共享组件拥有的
header/gutter geometry，不改变 document、editing 或 row-source contract。

`DiffDocument` 的输入
验证、资源限制和取消在进入本 crate 前由 `zeta-diff` 完成。组件只根据调用方提供的不可变
snapshot 生成 `UiScene`。

产品宿主必须：

- 为每个文件保存独立的 document identity 与 editor viewport；
- 为 `MultiDiffEditorItem` 提供已排序的 changed-file snapshot，并保留整体
  `zeta-ui::ScrollState`；
- 保存每文件 `DiffEditorState`，并用 `fold_controls` 发布的 identity 与 bounds 路由未修改区间
  的展开/收起输入；
- 把滚轮、平台按键和 IME 事件转换为 `CodeEditorCommand` / `TextInputCompositionEvent`；
- 用 `caret_bounds` 同步平台 IME candidate area，并由 host 控制 caret blink；
- 在接入异步 syntax 或 diff 计算时丢弃不再匹配当前 document revision 的结果。

`zeta-native` 是当前 GPU presentation host；`zeta-tui` 不依赖本 crate，而是直接消费
`zeta-diff` 并拥有自己的 Ratatui projection。

## 测试、修改影响与限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-editor
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-editor --all-targets -- -D warnings
```

测试覆盖 LF/CRLF/CR 行索引、多行 Unicode 编辑、跨行列导航、选择替换、有界撤销/重做、IME
预编辑与提交、语法前景 token、caret/selection/composition 绘制、viewport、Tab/Unicode 列宽和
无效 UTF-8 range，DiffEditor 的双 pane 映射、Unified 单列映射、字符级高亮和长未修改区间
展开/收起，以及 MultiDiffEditor 的多文件 section、每文件 fold-control identity、整体纵向滚动与
超大单文件的紧凑行映射与 row-level viewport culling。

修改 `CodeEditorRowSource` 或 `CodeEditorRow` 必须同步检查普通文档和 `DiffSideRows`；修改显示列
计算必须同步检查 text、horizontal scroll 与 inline decoration；修改 CodeEditor layout 必须同步
检查 DiffEditor 两侧 geometry 和 pointer mapping。若 DiffEditor 再次直接绘制代码行、行号或
字符范围，表示共享 CodeEditor ownership 已经漂移。若 `MultiDiffEditor` 绕过 `DiffEditor`
直接生成左右代码行，表示多文件组合层的 ownership 已经漂移。

当前 history 使用完整 snapshot 且不做 typing coalescing；syntax highlighter 是同步逐行 contract，
异步 parser 的 revision binding 仍由 host 负责。clipboard command、自动缩进、多光标、find/replace、
语言级函数/类型代码折叠、minimap、diagnostics 和 EditorHost 尚未完成。Unified DiffEditor 已拥有
纯 diff 语义的未修改区间折叠；它不会把这套规则放进普通 CodeEditor。MultiDiffEditor section 高度随 diff row
数量增长；完全不可见的文件会被剔除，部分可见的超大单文件只投影外层 viewport 内的行。
section 折叠仍由后续宿主接线完成。
