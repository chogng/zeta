# 编辑器核心：Native Rust 与 Alpha TypeScript 边界

> 状态：Current。`zeta-editor-core` 是 Zeterm Native editor 的纯 Rust document core；Alpha 是 Zeta
> Renderer 内独立的 TypeScript editor，不通过 WASM 或 App Server RPC 复用该同步 core。Rust 实现契约见
> [`zeta-editor-core`](../zeta-rs/editor-core/README.md)，app presentation 见
> [`zeta-editor`](../zeterm/editor/README.md)，Alpha 实现见
> [`desktop/src/zeta/editor/text-engine.md`](../desktop/src/zeta/editor/text-engine.md)。

## 当前所有权

| 能力 | Alpha（Zeta TS Renderer） | Zeterm（Native Rust） |
| --- | --- | --- |
| 文本存储、transaction、version | `TextModel` / `PieceTreeTextBuffer` | `EditorCoreDocument` / `CodeEditorDocument` |
| undo/redo 与 typing grouping | `TextModelHistory` | `EditorCoreDocument` + Native command policy |
| selection、tracked range、IME | Alpha common/browser | `zeta-editor` |
| layout 与 presentation | DOM、CSS、virtual viewport | `zeta-ui`、`zui`、GPU scene |
| 文件、LSP、workspace search | 异步调用 Rust App Server service | Native product adapter 按需组合 Rust crate |

两套 editor 共享产品语义和 conformance 目标，但不共享同步运行时状态。Alpha 的输入必须在 Renderer 当前事件
循环内完成；逐键 IPC、WASM 双写或远端 undo authority 都会破坏 IME、selection 和渲染一致性。Zeterm 则直接链接
Rust crate，不需要 Browser transport。

## Alpha 与 Rust 后端的数据流

```text
Browser input
  → Alpha TextModel 同步提交
  → versioned TextModelChange
     ├─ 同步更新 selection / tracked ranges / viewport
     ├─ 异步同步前端 language Worker
     └─ 异步发送 Rust file / LSP / search service

Rust result(resource + document version + request identity)
  → Alpha freshness gate
     ├─ current: result store / decoration / widget
     └─ stale: discard
```

Rust 后端适合拥有文件读写与冲突检测、language-server 生命周期和 document sync、workspace search、Git/diff、
formatting、rename、code action 和 parser-grade 分析。Alpha 始终拥有这些结果的交互状态与 presentation，并通过自己
的 `TextModel.applyEdits` 应用后端返回的 edits。

## `zeta-editor-core` 的 Native 契约

`zeta-editor-core` 接受 UTF-16 code-unit range、revision-bound atomic edit 和显式 post-selection，拒绝 stale
revision、代理对中间 offset、重叠 edit 和无效 selection。Native `CodeEditorDocument` 用 persistent core 持有
committed text、selection、revision 与 history；Native text projection 只服务 line index、syntax、folding 和绘制。

该 crate 不依赖 `zui`、`zeta-ui`、Native host、DOM、文件或 transport。若未来出现第二个真实 Rust consumer，应该
直接依赖其纯 Rust API；不得为 Alpha 的同步输入路径预先增加 WASM 或 App Server adapter。

## 长期不变量

- Alpha 的同步 text/history/selection authority 在 TypeScript Renderer。
- Rust App Server 结果必须携带并比较 Alpha document version；后端不可阻塞输入。
- Zeterm Native 直接消费 Rust editor/core/UI crate，不绕经 Electron 或 Browser API。
- `zeta-editor-core` 不依赖 presentation 或 transport；Alpha 不依赖 `zeta-editor-core`。
- 只有基准与真实消费者证明必要时才提取共享 wire contract，不能以潜在复用代替当前所有权。
