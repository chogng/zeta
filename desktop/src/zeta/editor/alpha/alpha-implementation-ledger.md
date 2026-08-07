# Alpha Editor 实现台账

本文件是 `editor-architecture.md` 的实现台账。架构文档描述目标分层和迁移原则；本文件按 VS Code `src/vs/editor/contrib` 的 feature 名称列出 Alpha 当前的 canonical 文件、职责、装配入口和边界。新代码必须先找到一行 owner，再决定文件位置。这里的 `Current` 指已有可调用实现，`Partial` 指实现存在但仍有明确的产品边界，不能把 `Planned` 写成“目录已经创建”。

## 1. 装配入口

| 入口 | 职责 |
| --- | --- |
| `browser/editorSession.ts` | 一个编辑器实例的唯一装配点；创建 model reference、selection、folding、syntax/token、completion、viewport、input 和 contrib controllers。 |
| `browser/editorPane.ts` | Workbench pane 生命周期；只把 host 的 resource/model contract 传给 session，不实现编辑语义。 |
| `browser/editorInput.ts` | Workbench `EditorInput` 匹配和语言 identity adapter；不进入同步 model/core。 |
| `browser/diffEditorInput.ts` | Workbench 双资源 diff input 和 synthetic tab identity；不创建 diff model，也不计算 diff。 |
| `browser/browserEditorSession.ts` | TextMate grammar readiness、syntax Worker 和 completion Worker 的 browser adapter。 |
| `editor.api.ts` | DOM-free 的 Alpha text-model 程序化 API；不加载 Workbench、DOM 或 contribution。 |
| `editor.all.ts` | Alpha 对产品入口公开的 contribution bundle；加载 editor browser contribution。 |
| `editor.main.ts` | 完整 Alpha 入口；组合 `editor.all.ts` 与 `editor.api.ts`。 |
| `editor.worker.start.ts` | Alpha dedicated language worker 的统一启动协议；syntax 与 completion worker 使用它建立 canonical wire port。 |
| `contrib/editor.contribution.ts` | 注册 Alpha code/diff pane，并强制注入 Workbench text-file 与 Rust diff adapter；生产环境不提供 fallback。 |
| `browser/widget/codeEditor/codeEditorWidget.ts` | 组合 viewport、输入、键盘导航、pointer selection 和 text drop；不拥有语言功能。 |
| `browser/widget/diffEditor/diffEditorWidget.ts` | 消费 `common/diff/diffModel.ts` 的只读 side-by-side projection；不计算 diff。 |

## 2. 公共同步内核

### 2.1 `common/core`

| 文件 | 职责 | 禁止依赖 |
| --- | --- | --- |
| `position.ts` / `range.ts` / `selection.ts` | 0-based line、UTF-16 column、end-exclusive range、anchor/active direction。 | model、DOM、Workbench、Rust DTO |
| `textChange.ts` / `editOperation.ts` | LF-normalized text change、transaction reason、原子 edit value。 | history 执行、selection controller |
| `text/*.ts` | 纯 text length、position/offset transformer 和 `TextModelLineSource`。 | 具体 `TextModel` 反向依赖 |
| `edits/*.ts` / `ranges/*.ts` | array、line、string、length edit 和 range mapping 算法。 | DOM geometry、语言 provider |
| `misc/*.ts` / `2d/*.ts` | EOL、indentation、RGBA、defaults 和通用几何值。 | feature 语义 |
| `wordHelper.ts` / `textSegmentation.ts` | grapheme、word、UTF-16 边界和字符分类基础规则。 | editor instance 状态 |

### 2.2 `common/model`

| 文件 | 职责 |
| --- | --- |
| `textModel.ts` | Piece Tree 之上的同步文档权威：文本、版本、原子 transaction、history、snapshot、change event。 |
| `pieceTreeTextBuffer/{pieceTreeTextBuffer,pieceTreeBase,pieceTreeSnapshot}.ts` | 文本存储、节点统计和 snapshot segment 读取；不改变 `TextModel` contract。正确性测试位于 `test/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.test.ts`。 |
| `editStack.ts` / `historyCoalescing.ts` | 文档 undo/redo、typing merge 和 history budget。 |
| `textModelSearch.ts` | literal、regex、whole-word、wrap 和 version-pinned search。 |
| `trackedRange.ts` / `decorationCollection.ts` | tracked range stickiness、decoration owner 和 model change 映射；不包含 CSS/severity。 |

### 2.3 `common/cursor` 与 `common/commands`

| 文件 | 职责 |
| --- | --- |
| `editorSelectionController.ts` | 一个 editor instance 的 selections、composition、cursor-only history 和 command execution。 |
| `cursorNavigation.ts` / `caretOperations.ts` | grapheme/caret、line/page/document navigation。 |
| `cursorTypeOperations.ts` / `cursorDeleteOperations.ts` / `cursorWordOperations.ts` | 输入、删除、word 删除产生 `EditorEditCommand`。 |
| `wordOperations.ts` / `wordPartOperations.ts` / `wordBoundary.ts` | word、camelCase、acronym、digit、separator 的 DOM-free boundaries。 |
| `columnSelection.ts` / `cursorTranspose.ts` / `cursorOvertype.ts` | column、transpose、overtype policy。 |
| `editorCommand.ts` / `editorEditCommand.ts` | ordered non-overlapping edits 与 selections-after 的 canonical command contract。 |
| `gotoLocation.ts` / `selectionText.ts` / `textSearchCommands.ts` | line/column/offset 解析、selection text 和 isolated search replace。 |

这些文件不读取键盘，不监听 DOM，不等待 IPC；browser controller 只能调用它们。

## 3. 语言、token 和服务边界

| owner | canonical 文件 | 责任 |
| --- | --- | --- |
| language base | `common/languages/languageConfiguration.ts`、`languageId.ts`、`languageLexical*.ts` | language configuration、lexical context 和纯 provider module contract。 |
| syntax | `common/languages/syntax/*.ts` | token/diagnostic lane、Worker wire、request freshness、delta 和 result acceptance。 |
| completion | `common/languages/completion/*.ts` | completion catalog、resolve wire、completion result、word provider。 |
| results | `common/languages/languageResults.ts`、`languageResultStore.ts`、`languageRequestCoordinator.ts` | diagnostic value、versioned store 和 stale-result gate。 |
| token | `common/tokens/languageTokens.ts`、`languageTokenLineIndex.ts` | token value/delta/normalization 和 sparse line index。 |
| tokenization contrib | `contrib/tokenization/common/tokenizationTextModelPart.ts`、`browser/tokenizationController.ts` | 将 token index 作为独立 model part 暴露给 browser/view；不生产 token。 |
| semantic tokens contrib | `contrib/semanticTokens/common/semanticTokens.ts`、`browser/semanticTokenPresentation.ts` | 规定 token source contract；presentation 只转换成稳定的 Alpha CSS vocabulary。 |
| frontend service | `common/services/languageService.ts` | provider registry、feature factory 和 per-model service；不暴露 Workbench transport 或 generated Rust DTO。 |
| runtime adapter | `browser/services/browserTextResourceStore.ts`、`browser/services/browserTextModelService.ts`、`browser/services/rustDiffComputationService.ts`、`browser/services/rustSyntaxFactsService.ts` | Workbench text file / Rust App Server 到 Alpha contract 的薄适配；Rust syntax facts 只经 revision gate 进入 token、diagnostic、symbol 和 folding consumer。 |

## 4. 已装配的编辑 contrib

### 4.1 输入、选择和文本操作

| feature | 文件 | 当前职责 |
| --- | --- | --- |
| `anchorSelect` | `contrib/anchorSelect/browser/anchorSelectController.ts` | Ctrl/Cmd+K 建立 editor-local anchor；capture keyboard navigation 生成 range selection，不改 model。 |
| `smartSelect` | `contrib/smartSelect/common/smartSelect.ts`、`browser/smartSelectController.ts` | 按 word、enclosing pair、line、document 层级扩展；不把 AST 强塞进 model。 |
| `inPlaceReplace` | `contrib/inPlaceReplace/browser/inPlaceReplaceController.ts` | 当前 selection 的 next/previous occurrence replacement，edit 通过 `createEditorEditCommand`。 |
| `multicursor` | `contrib/multicursor/common/occurrenceSelection.ts`、`browser/*.ts` | occurrence/cursor add-remove、dedupe 和 pointer/keyboard routing。 |
| `linesOperations` | `contrib/linesOperations/common/lineJoin.ts`、`browser/*.ts` | join、indent、duplicate、move、delete line groups。 |
| `comment` | `contrib/comment/common/*.ts`、`browser/*.ts` | language configuration 驱动的 line/block comment command。 |
| `clipboard` | `contrib/clipboard/common/clipboard.ts`、`browser/*.ts` | copy/cut/paste、line policy、URI/text provider 和 safe rich text。 |
| `dnd` / `dropOrPasteInto` | 各自的 `common`/`browser` | editor 内拖动 selection 与外部 file/text/URI drop；不可混入 pointer selection。 |
| `snippet` / `suggest` | `contrib/snippet/common/*.ts`、`contrib/suggest/{common,browser}/*` | parser、placeholder session、completion trigger/filter/resolve/widget。 |
| `wordWrap` / `transpose` / `insertFinalNewLine` | 各 feature 的 browser/common 文件 | 只改变 editor presentation 或 save boundary，不改变 model 默认语义。 |

### 4.2 语言和代码理解

| feature | canonical 文件 | 当前职责 |
| --- | --- | --- |
| `bracketMatching` | `contrib/bracketMatching/{common,browser}/*` | bracket match、pair edit、enter、navigation、colorization。 |
| `folding` | `contrib/folding/browser/*` | provider/indent ranges、tracked fold state、hidden lines、gutter presentation。 |
| `find` | `contrib/find/browser/findController.ts` + `common/model/textModelSearch.ts` | search/replace widget 和 selection scope；regex semantics 留在 model query。 |
| `wordHighlighter` | `contrib/wordHighlighter/{common,browser}/*` | current word occurrence query、decoration owner、renderer presentation。 |
| `gotoError` | `contrib/gotoError/common/diagnosticDecorations.ts`、`browser/gotoError.ts`、`browser/diagnosticOverviewRuler.ts`、`browser/languageDiagnosticPresentation.ts` | diagnostic decoration、navigation、overview ruler、hover data。 |
| `hover` | `contrib/hover/{common,browser}/*` | provider hover 与 diagnostic hover widget。 |
| `documentSymbols` / `gotoSymbol` | `contrib/documentSymbols/common/documentSymbols.ts`、`contrib/gotoSymbol/{common,browser}/*` | versioned symbol provider、flatten/query 和 symbol quick navigation。 |
| `links` | `contrib/links/{common,browser}/*` | deduplicated links、pointer target；open 始终交给 host callback。 |
| `codeAction` | `contrib/codeAction/{common,browser}/*` | result/resolve/picker/workspace edit；command 执行不归 service。 |
| `format` / `rename` | 各 feature 的 common/browser 文件 | provider request、freshness、input 和 canonical edit application。 |
| `inlayHints` / `inlineCompletions` / `parameterHints` | 各 feature 的 common/browser 文件 | inline projection、ghost text、signature help 和 key routing。 |
| `linkedEditing` | `contrib/linkedEditing/{common,browser}/*` | provider ranges 通过 tracked range 和 model transaction 同步。 |
| `codelens` | `contrib/codelens/{common,browser}/*` | versioned lens、resolve、inline button；host owns command execution。 |
| `colorPicker` | `contrib/colorPicker/{common,browser}/*` | color range/presentation contract、native color picker 和 text edit。 |

### 4.3 视图、导航和辅助 UI

| feature | 文件 | 当前职责 |
| --- | --- | --- |
| `editorState` | `contrib/editorState/{common,browser}/*` | focus、model version、selection、scroll 的 editor-local observable state。 |
| `contextmenu` | `contrib/contextmenu/browser/contextMenuController.ts` | editor hit-test 后把 context menu request 交给 host。 |
| `diffEditorBreadcrumbs` | `contrib/diffEditorBreadcrumbs/browser/diffEditorBreadcrumbs.ts` | diff hunk 索引和 reveal；不参与 Rust diff computation。 |
| `floatingMenu` | `contrib/floatingMenu/browser/floatingMenuController.ts` | selection anchor 的可选动作菜单；action callback 属于调用方。 |
| `fontZoom` | `contrib/fontZoom/browser/fontZoomController.ts` | per-editor zoom、line height 和 font measurement invalidation。 |
| `gpu` / `longLinesHelper` | `contrib/gpu/browser/gpuRenderer.ts`、`contrib/longLinesHelper/browser/longLinesHelper.ts` | capability/measurement budget；viewModel 不依赖 WebGL。 |
| `middleScroll` | `contrib/middleScroll/browser/middleScrollController.ts` | middle-button panning，独立于 pointer selection。 |
| `quickAccess` / `readOnlyMessage` | 各 feature 的 browser 文件 | editor-local go-to-line 与 readonly feedback，不拥有 Workbench global quick open/permission。 |
| `peekView` / `zoneWidget` | `contrib/peekView/browser/peekViewWidget.ts`、`contrib/zoneWidget/browser/zoneWidget.ts` | anchored transient surface 和其生命周期/布局容器。 |
| `placeholderText` | `contrib/placeholderText/browser/placeholderTextController.ts` | empty model presentation，不写入 model。 |
| `sectionHeaders` / `stickyScroll` | 各 feature 的 browser/common 文件 | folding hierarchy 的 header marking/sticky projection。 |
| `symbolIcons` | `contrib/symbolIcons/browser/symbolIconsController.ts` | document symbol kind 到稳定小图标的 presentation。 |
| `toggleTabFocusMode` | `contrib/toggleTabFocusMode/browser/toggleTabFocusModeController.ts` | Tab editor insertion 与 browser focus traversal 的状态路由。 |
| `unicodeHighlighter` / `unusualLineTerminators` | 各 feature 的 common/browser 文件 | invisible/bidi/confusable 和非标准 separator decoration；不修改 canonical LF model。 |
| `message` / `inlineProgress` | 各 feature 的 browser 文件 | editor-local transient status/progress，不替代 Workbench notification。 |

## 5. session 装配顺序和依赖方向

```text
TextModelReference
  -> EditorSelectionController
  -> FoldingModel / HiddenRangeModel
  -> SyntaxService
  -> TokenizationTextModelPart / SemanticTokens
  -> Language completion + snippet session
  -> CodeEditorWidget(view + input)
  -> synchronous editing contributions
  -> language UX contributions
  -> transient view/UI contributions
```

允许的方向：

```text
common/core -> common/model -> common/services / common/languages / common/tokens
browser/view -> common model snapshots + viewModel projections
contrib/common -> common/core/model/services
contrib/browser -> contrib/common + browser/view/input + host callbacks
Workbench/Electron -> browser adapters -> Alpha contracts
Rust App Server -> browser adapter -> Alpha async service contract
```

禁止的方向：

- `common` 反向 import Workbench、Electron、DOM 或 generated Rust DTO。
- Rust/IPC 进入键盘、IME、selection、TextModel transaction 热路径。
- contrib controller 把 provider transport、model edit semantics 或 host notification 复制一份。
- 通过空目录、空 controller、`index.ts` barrel 或 fallback 假装 feature 已完成。

## 6. 当前验证清单

- Alpha 目录没有 `index.ts` barrel。
- Renderer TypeScript 检查已经通过 Alpha 新增代码；App Server Session mutation 已统一走 canonical `session/request`，Thread 读取与订阅使用 Session-scoped RPC。
- 每个新增 common feature 至少有可独立运行的纯函数边界；已补 smart-select、Unicode highlighter、sticky-scroll 和 tokenization model-part tests。
- 新增 UI style 由对应 feature 的 `browser/media/*.css` 持有；Workbench host 不通过深层 selector 覆盖 editor internals。
- Rust diff 仍是生产强依赖：`browser/services/rustDiffComputationService.ts` 缺少 App Server transport 时显式失败，不存在 production fallback。
