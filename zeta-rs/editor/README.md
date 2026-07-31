# `zeta-editor`

> 本 README 是代码编辑器与差异编辑器 presentation 的 crate-level canonical contract。
> 跨 crate 的依赖与产品宿主边界见
> [`docs/zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md)；`zeta-diff` 算法契约见
> [`diff/README.md`](../diff/README.md)；异步结构分析与 revision binding 见
> [`docs/syntax-analysis.md`](../../docs/syntax-analysis.md)。

`zeta-editor` 拥有 Native UI 使用的多行代码编辑模型、caret/selection、键盘命令、有界
undo/redo、IME composition、语法 token、代码行与视口绘制，以及由两个代码视口组成的并排差异
展示和纵向组合多个文件差异的 `MultiDiffEditor`。它不读取文件、不执行 Git、不计算 diff，也
不拥有 changed-file collection、活动 EditorHost 或平台事件。

## 所有权与接口

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `CodeEditor` | public | 绘制可见代码行、caret/selection、preedit、syntax token、gutter 与 decoration |
| `CodeEditorDocument` | public | 保存文本、行 range、selection、composition、syntax snapshot 与 undo/redo |
| `CodeEditorCommand` | public | 表达插入、换行、Unicode navigation、选择、删除与 undo/redo |
| `CodeEditorSyntaxHighlighter` | public trait | 把单行文本同步投影为 UTF-8 byte-range foreground token |
| `CodeEditorRowSource` | public trait | 惰性提供稳定 visual row；普通文档和 diff projection 共用 |
| `CodeEditorRow` | public | 表达真实代码行或对齐 placeholder，以及本帧 decoration |
| `CodeEditorViewport` | public | 保存首个可见行和横向显示列，并执行有界滚动 |
| `CodeEditorStyle` | public | 拥有代码 surface、header、gutter 与文本的浅色 presentation token |
| `DiffEditor` | public | 组合两个 `CodeEditor`、同步纵向 viewport，并绘制中央 divider |
| `DiffEditorState` | public | 保存共享首行与两侧独立横向显示列 |
| `MultiDiffEditor` | public | 把多个文件标题和 `DiffEditor` section 组合为一个纵向裁剪 surface |
| `MultiDiffEditorItem` | public | 为一帧借用文件名、`DiffDocument`、两侧标签和该文件的 `DiffEditorState` |
| `zeta-ui::ScrollState` | delegated | 保存 MultiDiffEditor 整体 logical-pixel offset；clamp 与 transition 由通用滚动基座执行 |
| `MultiDiffEditorStyle` | public | 拥有文件 header、section 间距与嵌套 DiffEditor 样式 |
| `DiffSideRows` | private | 把 `DiffDocument` 的一侧惰性转换为 `CodeEditorRow` |
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
├─ DiffSideRows(original) → CodeEditor
├─ DiffSideRows(modified) → CodeEditor
└─ divider

MultiDiffEditor::paint
├─ ScrollView::draw
│  ├─ viewport clip / content origin
│  └─ hover/active/fading scrollbar chrome
└─ visible MultiDiffEditorItem
   ├─ file header
   └─ DiffEditor → two CodeEditor panes
```

`CodeEditorRowSource` 实现必须在一个 presentation frame 内保持 row ordering 和 line-number
identity 稳定，并只为请求的可见 index 构造行。普通文档返回真实编号行；差异投影可以返回没有
line number 的 placeholder，但不能把 placeholder 写入 authoritative document。行内高亮使用
UTF-8 byte range；越界或不落在字符边界上的 range 会被忽略，不得造成绘制 panic。

## 执行、失败与宿主义务

`CodeEditor`、`DiffEditor` 和 `MultiDiffEditor` 构造与绘制没有 I/O 或独立 error channel。编辑命令以 Unicode
grapheme boundary 修改 committed text；CRLF 在删除时作为一个换行边界处理。history 最多保留
100 个完整 snapshot，新编辑会清空 redo；当前不合并连续输入。IME preedit 与 committed text
分离，只有 Commit 建立一个可撤销 checkpoint，Cancel 不修改文档。`CodeEditor::caret_bounds`
向平台宿主提供候选框锚点，`text_position_at` 与 `CodeEditorDocument::set_selection` 支持指针选择。

`DiffDocument` 的输入
验证、资源限制和取消在进入本 crate 前由 `zeta-diff` 完成。组件只根据调用方提供的不可变
snapshot 生成 `UiScene`。

产品宿主必须：

- 为每个文件保存独立的 document identity 与 editor viewport；
- 为 `MultiDiffEditorItem` 提供已排序的 changed-file snapshot，并保留整体
  `zeta-ui::ScrollState`；
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
无效 UTF-8 range，DiffEditor 的双 pane 映射和字符级高亮，以及 MultiDiffEditor 的多文件
section、裁剪和整体纵向滚动。

修改 `CodeEditorRowSource` 或 `CodeEditorRow` 必须同步检查普通文档和 `DiffSideRows`；修改显示列
计算必须同步检查 text、horizontal scroll 与 inline decoration；修改 CodeEditor layout 必须同步
检查 DiffEditor 两侧 geometry 和 pointer mapping。若 DiffEditor 再次直接绘制代码行、行号或
字符范围，表示共享 CodeEditor ownership 已经漂移。若 `MultiDiffEditor` 绕过 `DiffEditor`
直接生成左右代码行，表示多文件组合层的 ownership 已经漂移。

当前 history 使用完整 snapshot 且不做 typing coalescing；syntax highlighter 是同步逐行 contract，
异步 parser 的 revision binding 仍由 host 负责。clipboard command、自动缩进、多光标、find/replace、
折叠、minimap、diagnostics 和 EditorHost 尚未完成。MultiDiffEditor section 高度随 diff row
数量增长并剔除完全不可见的文件；部分可见的超大单文件 section 尚未做 row-level virtualization。
section 折叠仍由后续宿主接线完成。
