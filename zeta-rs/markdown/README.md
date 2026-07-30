# `zeta-markdown`

> 本 README 是 Native Markdown 解析、布局和 presentation 的 crate-level canonical
> contract。跨 crate 的 Rust Workspace 边界见
> [`docs/zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md)；底层 scene、富文本和字体
> shaping contract 见 [`ui/README.md`](../ui/README.md)。

`zeta-markdown` 把有资源上限的 CommonMark/GFM 输入解析为只读文档 snapshot，并通过
`zeta-ui` 的富文本、矩形和裁剪 primitive 生成 Native Markdown 组件。它拥有标题、段落、
粗体、斜体、行内代码、链接 presentation/命中 geometry、列表、任务项、引用、代码块、二维表格、
删除线、分隔线、
通用滚动 snapshot 消费和主题 token；不拥有消息 identity、网络图片、语法高亮、链接打开、平台输入、
持久化或产品生命周期。

## 所有权与接口

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `MarkdownDocument::parse` | public | 在 4 MiB 输入、100,000 blocks 和 64 层嵌套上限内建立不可变文档 snapshot |
| `MarkdownError` | public | 区分输入过大、嵌套过深和 block 数超限 |
| `MarkdownLayoutEngine` | public | 保留可复用 font shaping state，把文档、bounds、`ScrollState` 和 style 投影为一帧 `Markdown` |
| `Markdown` / `MarkdownLink` | public | 保存当前帧可见 primitive 与 viewport-clipped link hit fragments；只返回未受信任 destination，不执行激活 |
| `MarkdownStyle` | public | 提供正文、标题、链接、代码、引用、列表和表格的 presentation token 与 geometry |
| `document::DocumentBuilder` | private | 消费 `pulldown-cmark` event，维护 inline format、quote/list/table nesting 和当前 block |
| `table::TableBuilder` | private | 把 table event 序列组装成保留 row/cell 边界的单个 `MarkdownTable` block |
| `document::push_run` | private | 合并相邻同样式 run，避免解析事件制造无意义的 span 碎片 |
| `inline_layout::layout_inline` | private | 调用 `TextLayoutEngine::layout_spans`，从同一次 shaping 取得 span fragments，并生成 code/link/strikethrough decoration |
| `table_layout::layout_table` | private | 从全部 cell 的 intrinsic width 计算列宽，再按列宽 wrapping cell，并保存 row height |
| `component::ProjectedBlock` | private | 保存 shaping 后的单个 text/code/table/rule block 高度与宽度 |
| `Markdown::emit` | private | 把自然文档坐标减去有效 viewport offset，剔除不可见 block 并生成 scene primitive |

```text
MarkdownDocument::parse
└─ pulldown_cmark::Parser
   └─ DocumentBuilder::consume
      ├─ inline format/link stack
      ├─ quote/list context
      ├─ TableBuilder → MarkdownTable rows/cells
      └─ MarkdownBlock

MarkdownLayoutEngine::layout
├─ MarkdownBlock + MarkdownStyle
├─ zeta_ui::TextLayoutEngine
│  └─ TextLayout size + per-span wrapped/BiDi visual fragments
├─ layout_inline → inline backgrounds/decorations/link hit fragments
├─ layout_table → intrinsic column sizing → wrapped cell layout
├─ content_height + clamped zeta_ui::ScrollState
└─ Markdown::emit
   ├─ PaintRect: code/table/quote/rule/inline decoration
   ├─ MarkdownLink: viewport-clipped hit fragments
   └─ TextBlock/TextSpan: wrapped rich text

Markdown::paint
└─ UiScene::with_clip(viewport bounds)
```

`MarkdownLayoutEngine` 必须由 host 复用；每次 document、bounds、style 或 scroll snapshot 变化时重新
生成 immutable `Markdown`。`Markdown` 不保存 parser 或 font system，也不接受输入事件。
`zeta_ui::ScrollState` 是 product-owned retained state，组件只消费其 snapshot；wheel normalization、
scrollbar 和 input routing 继续复用 `zeta-ui` 的通用 scroll contract。

## 解析、信任与显示语义

解析启用 CommonMark、GFM block quote、table、strikethrough 和 task-list 扩展。Soft break
折叠为空格，hard break 保留换行；fenced code 的 info string 作为语言标签显示。表格保留
row/cell 边界，先以所有 cell 的 intrinsic width 分配整表列宽，再在各列内部独立换行；header
使用独立背景和粗体，网格线只绘制一次。

Markdown 输入是不可信文本：

- raw/inline HTML 只作为可见字符串保留，不解释 DOM、脚本、attribute 或 URL；
- 图片只显示 alt text，不发起网络或 data-URL 解码；
- link destination 进入 `MarkdownLink` 的同源命中 geometry，但仍是未受信任字符串；
- crate 没有文件、网络、WebView 或 command 依赖，因此解析本身不能触发外部副作用。

host 增加链接激活时，必须使用独立的安全 URL policy，并从 `Markdown::link_at` 返回的同源命中
geometry 路由；不能把 parser destination 直接交给 shell。增加图片时也必须由 product image policy
显式授权和加载，不能让 Markdown parser 拥有网络。

## 测试、修改影响与限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-markdown
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-markdown --all-targets -- -D warnings
bazel test //zeta-rs/markdown:markdown-unit-tests
```

测试覆盖 block/inline parsing、列表与任务 marker、引用 nesting、fenced code language、表格
header/body、raw HTML 可见文本、输入上限、富文本样式投影、代码/引用 surface、段落换行和
viewport clamp、wrapped span fragments、inline decoration/link hit geometry、二维表格列宽与
cell wrapping。修改 parser option 或 `DocumentBuilder` 时必须同步检查资源上限、HTML/image
信任边界和 block fixtures；修改 `ProjectedBlock`/spacing 时必须同步检查 content height、
offscreen culling 与 scroll clamp；修改 rich span contract 时必须同步检查 `zeta-ui::TextSpan`、
renderer validation 和 `ui/README.md`。

当前不支持 link activation/URL policy、图片加载、syntax highlighting、脚注、数学公式、
Markdown source 内声明的 table alignment、文本选择、复制、搜索、accessibility semantic tree
或 block-level retained cache。首次产品接入应由 `zeta-native` 或其他 host 绑定消息、滚动和
安全链接激活；这些能力不得反向进入 parser/document ownership。
