# 编辑器语法能力

> 状态：Rust `CodeEditor` 已内部接入 Rust、JSON、JSONC 与 Shell 的增量 tree-sitter
> 分析；Rust `DiffEditor` 已通过 retained `DiffEditorDocument` 内部维护两侧语法状态；Desktop
> Alpha 的 JSON/JSONC 继续使用编辑器内 TextMate provider。本文拥有跨编辑器的语法能力边界；底层解析契约见
> [`zeta-syntax` README](../zeta-rs/syntax/README.md)，Native 编辑器 API 见
> [`zeta-editor` README](../zeterm/editor/README.md)。

## 快速理解

语法高亮、折叠、outline 和 parse diagnostic 都是编辑器能力。产品层对外组合
`CodeEditor` 或 `DiffEditor`，只提供文档、语言、主题和平台输入；它不获得独立 syntax service，
也不通过 App Server 同步编辑器内部 revision。

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| Rust/JSON/JSONC/Shell grammar、query 与增量 tree | `zeta-syntax`，由 Rust `CodeEditor` 私有组合 | ✅ |
| 文本、selection、undo/redo、语言切换和 syntax token 生命周期 | `zeta-editor::CodeEditorDocument` | ✅ |
| Native 普通代码结构折叠、visible-row mapping 与 gutter control | `zeta-editor::CodeEditorDocument` / `CodeEditor` | ✅；宿主只转交点击 |
| Native Composer 的 Shell 高亮 | `CodeEditorDocument::from_text_with_language` / `set_language` | ✅ |
| Native 文件 document lifecycle | `zeta-text-file` + `FileEditorHost` / `FileEditorPane` | ✅；独立 crate 拥有 baseline/version/dirty/conflict，Native 已接通 Tab、Explorer load、save、关闭确认、外部重载/显式乐观覆盖、中心 Editor Surface 以及 keyboard/IME/pointer/clipboard/viewport 输入 |
| Native `DiffEditor` 两侧 syntax token 投影 | `zeta-editor::DiffEditorDocument` / `DiffEditor` | ✅；宿主只提交 diff 与 language |
| Alpha JSON/JSONC token | Alpha analysis provider + TextMate worker | ✅ |
| Alpha Rust token | 后续 Alpha 编辑器内 provider | 尚未完成 |
| App Server syntax RPC | 无 | ❌；不属于远程产品服务 |
| completion、type、definition/reference、rename | `zeta-lsp` + language server，经编辑器 language feature 接入 | 部分具备 |
| workspace symbol index | 后续 workspace index | 尚未完成；不属于单个 editor document |

## 一次 CodeEditor 编辑

```text
Native host
  └─ CodeEditorDocument::apply / apply_composition / replace_text
       ├─ 修改 authoritative editor text 与 history
       ├─ CodeEditorAnalysis::synchronize
       │  └─ zeta-syntax::SyntaxDocument::apply_edit
       └─ 同 revision snapshot
          ├─ token → line-relative CodeEditorSyntaxToken
          └─ folding range → editor-owned visible-row projection
             └─ CodeEditor::paint / fold_control_at
```

1. 宿主创建 `CodeEditorDocument` 时选择 `CodeEditorLanguage`，或在 mode 改变时调用
   `set_language`。
2. `CodeEditorDocument` 执行 Unicode-safe 编辑、IME commit、undo/redo 或全文替换。
3. 私有 `CodeEditorAnalysis` 计算单个 UTF-8 replacement，推进内部 revision，并让
   `SyntaxDocument` 复用旧 tree；解析失败时丢弃旧 analyzer，并从当前 authoritative text 重建。
4. 同 revision snapshot 被投影为逐行 token 与 source-row folding range；每个
   `CodeEditorDocument` 独立保存 collapsed state，并把 source row 映射为 visual row。
5. 绘制只读取 document 已确认的 token/fold projection，并从当前 `CodeEditorStyle` 解析颜色；
   主题变化不触发重解析。宿主把 `fold_control_at` 的结果原样交给 `toggle_fold_control`，不计算
   fold 或 hidden row。

调用方看不到 parser、tree、revision、edit batch 或 syntax transport。这些细节随编辑器文档一起
创建、编辑和释放，避免 model ID、连接所有权、重连恢复和 stale-result gate 泄漏到 Workbench。

## 所有权边界

| 能力 | `zeta-syntax` | `CodeEditor` / Alpha | 产品宿主 | App Server | `zeta-lsp` |
| --- | --- | --- | --- | --- | --- |
| grammar、query、tree-sitter tree | ✅ | 组合/消费 | ❌ | ❌ | ❌ |
| editor text、selection、language 与本地 revision | ❌ | ✅ | 选择初始资源与语言 | ❌ | 消费同步 |
| syntax token 与 parse facts | 计算 | ✅ 生命周期与展示 | ❌ | ❌ | ❌ |
| theme color、DOM/native geometry、fold UI state | ❌ | ✅ | 注入主题/布局 | ❌ | ❌ |
| 文件 dirty/save/conflict | ❌ | ❌ | 组合 `zeta-text-file` | 可提供独立文件 I/O capability | ❌ |
| type、completion、definition/reference、rename | ❌ | 交互入口 | 协调 | 可承载独立 LSP runtime | ✅ |
| workspace 扫描、watch 与 symbol index | ❌ | ❌ | 后续组合 | 可承载独立 workspace capability | 消费/提供事实 |

`zeta-syntax` 仍保持独立 crate，因为 grammar、query、资源上限和增量 tree 算法需要可测试的领域
边界；但它不是产品对外服务。`zeta-editor` 是当前唯一产品 consumer，Native 不直接依赖它。
tree-sitter 的输出是 concrete syntax facts，不是统一 AST，也不是 compiler/LSP semantic facts。

## Desktop Alpha

Alpha 同样只对外暴露编辑器与 versioned language-provider contract。当前 JSON/JSONC grammar 在
Alpha TextMate worker 中运行，浏览器 session 直接从 editor-local provider factory 创建分析
worker；Workbench 不注册 syntax service，Renderer API 和 App Server protocol 也没有 syntax
capability。

这次边界收敛删除了先前的 Rust App Server provider，因此 Alpha Rust 高亮当前尚未完成。后续若
需要 Rust tree-sitter，应实现 Alpha 拥有的 worker/provider，并保持 parser transport 私有；不能
恢复 `document/syntax/open|change|close` 作为产品 API。

## DiffEditor 与后续演进

`DiffEditorDocument` 接收已经计算完成的 `DiffDocument` 和一个 `CodeEditorLanguage`，内部重建
original/modified 精确文本，并各自持有 `CodeEditorDocument`。双栏和 unified projection 根据
source line number 读取同一份 editor-owned token；Changes Pane 不知道 parser、revision 或 token。
语言切换只调用 `DiffEditorDocument::set_language`，不会产生 App Server 请求。

近期工作按以下顺序推进：

1. Alpha 按真实产品需要增加 editor-local Rust provider。
2. outline 和 parse diagnostics 只有出现具体 UI consumer 后才扩展编辑器公开 contract。
3. workspace index 只在真实 Files/Search consumer 建立后作为独立能力设计。

长期不变量是：产品层暴露编辑器，不暴露 parser RPC；编辑器拥有文档内语言能力，
`zeta-syntax` 拥有底层 syntax 算法，LSP/compiler 拥有跨文件语义事实。
