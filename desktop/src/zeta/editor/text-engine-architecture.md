# Aster Text Engine：VS Code Editor 基线的文件级架构与迁移手册

> 文档所有权：`desktop/src/zeta/editor` 中的行式文本 engine。
>
> 本文是 Aster 行式文本 engine 的开发架构文档和文件迁移清单。它规定文件命名、职责、依赖方向、当前实现状态和后续迁移顺序；统一目录与装配边界见 [`README.md`](./README.md)，行为细节、测试证据和已知限制见 [`text-engine.md`](./text-engine.md)。
>
> 状态：Current implementation + Proposed target。`Current` 表示当前代码和测试已经支持；`Partial` 表示能力存在但职责或文件位置仍需调整；`Planned` 表示目标已确定但尚未实现；`Non-goal` 表示明确不由 Aster editor 拥有。

> 实施台账：本文件保留迁移目标和 VS Code 对照表；当前实现、实际装配入口、测试证据和限制以 [`text-engine-implementation-ledger.md`](./text-engine-implementation-ledger.md) 为准。目标表中的旧 `Planned`/`Partial` 标记不覆盖台账中已经落地的实现。

## 快速理解

Aster 使用 VS Code editor 的成熟分区作为起点，但不复制 VS Code 的历史依赖关系。VS Code 官方将 editor 分成 `common`、`browser` 和可选的 `contrib`：`common` 与 `browser` 构成不依赖 Workbench 的编辑器核心，`contrib` 是可以独立装配或移除的编辑器功能。Aster 采用同样的产品边界，再加入严格的 model/view-model/runtime 依赖规则，以及 Rust App Server 的异步能力边界。

Aster 的同步权威只有 Renderer 内的 TypeScript model：输入、事务、selection、undo/redo、tracked range、decoration 和视图投影必须在当前事件循环内完成。Rust 只通过前端服务契约提供 diff、语言、文件、搜索和其他异步能力；它不能成为键盘输入或 IME 的远程依赖。

```text
common/core
      ↓
common/model
      ├── common/cursor
      ├── common/commands
      ├── common/viewLayout
      ├── common/viewModel
      └── common/languages / common/tokens
                         ↓
browser/view + browser/input + browser/services
                         ↓
Workbench host / App Server adapters / optional contributions
```

## 1. VS Code 基线与 Aster 的取舍

VS Code 的官方源码组织说明了这些 editor 边界：`vs/editor` 不应依赖 Node 或 Electron；`vs/editor/common` 与 `vs/editor/browser` 构成核心；`vs/editor/contrib` 是可选的编辑器能力；Workbench 负责宿主和产品级组合。当前基线以 VS Code `main` 分支的目录为参考，不把 VS Code 的源码当作 Aster 的运行时依赖。

- [VS Code Source Code Organization](https://github.com/microsoft/vscode/wiki/source-code-organization)
- [VS Code `src/vs/editor/common`](https://github.com/microsoft/vscode/tree/main/src/vs/editor/common)
- [VS Code `src/vs/editor/browser`](https://github.com/microsoft/vscode/tree/main/src/vs/editor/browser)
- [VS Code `src/vs/editor/contrib`](https://github.com/microsoft/vscode/tree/main/src/vs/editor/contrib)

### 文件命名规则

- Aster 中存在直接对应的 VS Code contrib 入口时，文件使用 VS Code 的 feature basename，例如 `gotoError.ts`、`indentation.ts` 和 `folding.ts`。
- 没有一一对应的 Aster 投影或 adapter 不强行伪装成 VS Code 文件：`diagnosticOverviewMarkers.ts` 表示 viewport-owned overview marker 聚合，`languageDiagnosticPresentation.ts` 表示语言诊断 presentation，`decorationCollection.ts` 表示 Aster 自己的 decoration owner collection。
- `*Controller.ts` 只在文件确实只拥有一个 browser controller 时保留；它不是所有 contrib 入口的统一后缀。功能入口、纯计算模块和具体 UI 控制器必须按各自职责命名。

| VS Code 设计 | Aster 采用方式 | 需要改变的地方 |
| --- | --- | --- |
| `common` / `browser` runtime split | ✅ 保留 | Aster 进一步禁止 common 依赖 Workbench transport |
| `model`、`cursor`、`viewModel` 的职责分区 | ✅ 保留 | 以依赖图而不是历史文件位置作为最终标准 |
| `contrib/<feature>/{common,browser}` | ✅ 保留 | 每个贡献点都要有清晰的 feature owner、装配入口和测试边界 |
| 全局 Service Identifier 和大量隐式 DI | 部分采用 | 只在跨宿主、可替换的 Editor contract 使用；普通依赖优先显式传入 |
| Workbench 服务直接参与编辑器实现 | ❌ 不采用 | 由 browser/workbench adapter 转换为 Aster-owned contract |
| legacy editor runtime/VS Code 兼容类型作为模型权威 | ❌ 不采用 | 兼容层只能位于 adapter，不能反向定义 Aster model |
| Rust/IPC 参与每次输入 | ❌ 不采用 | Rust 是异步计算和持久化能力提供者 |

## 2. 分层与所有权

### 2.1 `common/core`

`core` 是纯编辑器数学和协议值对象层。它可以依赖 `base/common` 的通用事件、生命周期、集合和基础类型，但不得依赖 `model`、`browser`、`language`、Workbench 或 App Server。

| 目标文件 | 职责 | 不负责 |
| --- | --- | --- |
| `common/core/position.ts` | 0-based 行索引和 UTF-16 列索引 | 文档内容存储 |
| `common/core/range.ts` | 有序、end-exclusive 文本范围 | tracked range 生命周期 |
| `common/core/selection.ts` | anchor/active 方向和 selection value | 某个 editor instance 的当前 selection |
| `common/core/textChange.ts` | 文本变化、版本原因和行尾规范化 | 持久化或 worker 传输 |
| `common/core/editOperation.ts` | 原子 edit 描述和历史分组 | 执行 edit |
| `common/core/edits/*` | offset、line、string、length edit 算法 | model history |
| `common/core/ranges/*` | line/column/offset/range mapping | DOM 坐标 |
| `common/core/text/*` | 文本长度、position/offset transformer、纯文本抽象 | 依赖具体 TextModel 的 helper |
| `common/core/textSegmentation.ts` | grapheme、word、UTF-16 边界 | 编辑器 selection 状态 |
| `common/core/wordHelper.ts` | 字符分类和单词边界基础规则 | model 查询和 UI |
| `common/core/2d/*` | 与编辑器无关的几何值对象 | viewport scroll policy |
| `common/core/misc/*` | EOL、indentation、颜色和通用小型值对象 | 领域服务 |

当前需要修正的依赖：`common/core/text/getPositionOffsetTransformerFromTextModel.ts` 不能继续让 core 反向依赖 `model`。它应改成接受 core-owned 的 `TextModelLineSource`，或迁移到 `common/model` 下的 adapter。

### 2.2 `common/model`

`model` 是 Aster 的同步文档内核。当前打开的 [decorationCollection.ts](./common/model/decorationCollection.ts) 属于此层：它拥有 decoration collection 和 tracked range 的关系，但不拥有 CSS、DOM 或语言语义。

| 目标文件 | 职责 | 关键 owner |
| --- | --- | --- |
| `common/model/textModel.ts` | 文本内容、版本、事务、变化事件和 snapshot | `TextModel` |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | Piece Tree 的文本和行索引存储 | `PieceTreeTextBuffer` |
| `common/model/pieceTreeTextBuffer/pieceTreeBase.ts` | Piece Tree 节点、旋转、合并和范围统计 | `PieceNode` |
| `common/model/pieceTreeTextBuffer/pieceTreeSnapshot.ts` | 不可变 snapshot segment 读取 | `TextBufferSnapshot` |
| `common/model/editStack.ts` | undo/redo、事务边界和历史预算 | `TextModelHistory` |
| `common/model/historyCoalescing.ts` | 输入历史合并和逆操作规范化 | history helpers |
| `common/model/textModelSearch.ts` | literal/regex/whole-word 文档搜索 | search functions |
| `common/model/trackedRange.ts` | edit 后的 range 映射和 stickiness | `TrackedRange` |
| `common/model/decorationCollection.ts` | 稳定 decoration ID、metadata 和 collection event | `TextDecorationCollection` |

`model` 允许依赖 `core`、`base/common/event` 和 `base/common/lifecycle`。它不得依赖 `ITextFileService`、DOM、language provider、Rust DTO 或任何 view presentation。

#### 2.2.1 与 VS Code `common/model` 的基线对照

当前 `../vscode/src/vs/editor/common/model` 基线包含 43 个文件。Aster 不把这棵目录
当作需要逐文件复制的运行时依赖，而是把它作为能力索引和命名基线。下面按职责族合并
文件；“当前（职责迁移）”表示能力已经存在，但 canonical owner 不在同一个相对路径；
“部分具备”只表示原基线中的部分语义或性能特征尚未覆盖。

| VS Code model 基线 | Aster canonical owner | 状态 | 对齐结论 |
| --- | --- | --- | --- |
| `textModel.ts` | `common/model/textModel.ts` | 当前 | 文本、版本、原子 transaction、snapshot、同步 change event 和文档 history 由 Aster 自己定义；不引入 VS Code public model 类型。 |
| `pieceTreeTextBuffer/pieceTreeBase.ts`、`rbTreeBase.ts` | `common/model/pieceTreeTextBuffer/pieceTreeBase.ts` | 当前（文件名对齐） | Aster 使用确定性 treap，而不是 VS Code 的 red-black tree；文件名用于对照，底层树算法仍由 Aster 自己实现。 |
| `pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | 当前（文件名对齐） | replace、行计数、snapshot 和阈值驱动的 compaction 已由 Aster storage owner 承担。 |
| `pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts` | 暂无直接 owner | 尚未采用 | 当前 `TextModel` 从规范化字符串构造；只有资源读取契约引入 streaming/builder 后，才建立对应 Aster contract。 |
| `textModelSearch.ts` | `common/model/textModelSearch.ts` | 当前 | literal、regex、whole-word、wrap 和 version-pinned search 保持在 model query 层。 |
| `editStack.ts` | `common/model/editStack.ts`、`historyCoalescing.ts` | 当前（文件名对齐） | 文档 undo/redo、typing merge 和 history budget 属于 Aster model；editor-instance selection undo 仍在 cursor/contrib。 |
| `intervalTree.ts` | `common/model/trackedRange.ts`、`decorationCollection.ts` | 部分具备 | tracked range 和 decoration 的语义契约已具备；当前索引是 collection 内的 `Map`，尚未提供 interval-tree 级别的大量 range 更新性能。 |
| `decorationProvider.ts` | `common/model/decorationCollection.ts`、`browser/view/*` | 当前（职责迁移） | model 只保存 range 和 caller metadata；CSS、severity、geometry 和 DOM presentation 留在对应 view/feature owner。 |
| `guidesTextModelPart.ts` | `contrib/folding/*`、`contrib/indentation/*` | 当前（职责迁移） | guides、folding ranges 和 hidden-line projection 是 feature/view-model 能力，不进入通用 TextModel part。 |
| `indentationGuesser.ts` | `common/core/misc/indentation.ts`、`contrib/indentation/*` | 当前（职责迁移） | 纯缩进算法和 editor presentation 分开；不为 VS Code 文件名建立空 model helper。 |
| `mirrorTextModel.ts` | `common/languages/languageWorkerDocumentMirror.ts` | 当前（职责迁移） | Worker mirror 是语言运行时边界，不是同步 TextModel 的第二个 owner。 |
| `fixedArray.ts`、`prefixSumComputer.ts`、`textModelStringEdit.ts`、`textModelText.ts`、`utils.ts` | `common/core/{edits,ranges,text,*}`、`common/tokens/*` | 当前（职责迁移） | 纯算法按调用者职责进入 core/text/ranges/tokens；Aster 不保留泛化的 model utility bucket。 |
| `textModelPart.ts` | `contrib/tokenization/common/tokenizationTextModelPart.ts`、`common/languages/*` 等显式 feature contract | 当前（职责迁移） | Aster 不建立一个拥有所有 model part 生命周期的通用基类；各 feature 明确拥有自己的 state、listener 和 dispose。 |
| `textModelTokens.ts` | `common/tokens/*`、`contrib/tokenization/*`、`common/languages/syntax/*` | 部分具备 | 版本化 token result、按行 sparse index 和 tokenization model-part contract 已具备；VS Code 的全部 background tokenizer/state backend 尚未一一覆盖。 |
| `tokens/{abstractSyntaxTokenBackend,annotations,tokenizationFontDecorationsProvider,tokenizationTextModelPart,tokenizerSyntaxTokenBackend}.ts` | `common/tokens/*`、`contrib/tokenization/*`、TextMate/syntax adapters | 部分具备 | token 生产由 Aster language/provider contract 决定；不得为了文件对齐把 Workbench tokenizer 或第三方 runtime 直接放进 model。 |
| `tokens/treeSitter/*` | `zeta-rs/syntax`、`platform/syntax`、`browser/services/rustSyntaxFactsService.ts` | 部分具备 | Rust owns bounded Tree-sitter parsing and UTF-16 DTO projection; Aster consumes revision-bound token, diagnostic, symbol, and folding facts for JavaScript/JSX, TypeScript/TSX, JSON, JSONC, Rust, and Shell through existing result stores and contribs. Broader language coverage remains future work. |
| `bracketPairsTextModelPart/*` | `contrib/bracketMatching/common/*`、`common/languages/languageLexical*`、对应 browser presentation | 当前（职责迁移） | lexical bracket matching、colorization、navigation、pair editing 已有独立 feature owner；Aster 当前没有 model-resident incremental bracket-pair tree。 |

这张表的用途是保持“可搜索的语义对齐”，不是承诺 Aster 复刻 VS Code 的内部数据结构。
因此，VS Code 新增文件时，先判断它属于现有 Aster owner、需要新增后端无关 contract、
还是明确不属于 Aster；不能仅因基线出现同名文件就在 `common/model` 增加副本。

#### 2.2.2 Model 能力审计结论

| 审计项 | 当前判断 | 下一步 |
| --- | --- | --- |
| 文本存储、transaction、version、snapshot、undo/redo、search | ✅ 当前 | 继续以 `TextModel.commitOffsetEdits` 和 `PieceTreeTextBuffer` 为唯一同步 mutation/storage boundary。 |
| tracked range / decoration 的大规模更新 | 部分具备 | 当前以语义/正确性测试守住 mapping；仅在真实产品性能预算失守时，用可复现的产品负载采样定位瓶颈，再在 `common/model` 内引入私有 interval index。 |
| bracket-pair 增量结构 | 部分具备 | 当前 lexical feature 已满足基础行为；如需大文件或 parser-grade parity，先定义增量失效和 snapshot contract，再决定是否新增 model backend。 |
| tokenization / semantic token state | 部分具备 | 保持 `common/tokens`、`contrib/tokenization` 和 language worker 的分层；Tree-sitter/AST 只作为明确 provider 方案推进。 |
| streaming/builder 输入 | 尚未采用 | 当前字符串 resolve contract 不需要它；资源层改为流式读取前，不新增 `pieceTreeTextBufferBuilder.ts`。 |

当前最明确的架构结论是：目录层面已经按 VS Code 语义可检索地对齐；剩余工作是
range 索引、增量 bracket/token backend 和 parser-grade provider 的能力审计，而不是
同步两棵源码目录。

### 2.3 `common/cursor`

`cursor` 只负责 editor instance 的光标和 selection 状态，以及把用户意图转换为 model edit。它不拥有 DOM input，也不直接注册 Workbench command。

| 目标文件 | 职责 |
| --- | --- |
| `common/cursor/editorSelectionController.ts` | 一个 editor instance 的 selection、cursor-only undo 和 selection transaction |
| `common/cursor/editorSelectionOperations.ts` | selection offset 校验、排序、结果长度和 selection 构造 |
| `common/cursor/editorComposition.ts` | IME composition 的 provisional range 和提交/取消语义 |
| `common/cursor/cursorNavigation.ts` | character、line、word、visual position 导航 |
| `common/cursor/cursorInsertion.ts` | 相邻位置、行首尾和多光标插入位置 |
| `common/cursor/cursorDeleteOperations.ts` | Backspace、Delete、行首尾和 grapheme-safe 删除 |
| `common/cursor/cursorTypeOperations.ts` | 普通输入、selection replacement 和 typing command |
| `common/cursor/cursorTypeEditOperations.ts` | 将 selection edit 转换为可执行 command |
| `common/cursor/cursorWordOperations.ts` | word/word-part 删除和选择操作 |
| `common/cursor/cursorOvertype.ts` | overtype 的范围计算和替换 |
| `common/cursor/cursorTranspose.ts` | grapheme-safe 字符交换 |
| `common/cursor/columnSelection.ts` | Alt+Shift rectangular selection |
| `common/cursor/wordBoundary.ts` | model 上的完整 word selection 边界 |
| `common/cursor/occurrenceSelection.ts` | 多 occurrence selection |
| `common/cursor/occurrenceHighlights.ts` | 当前 word/selection 的 occurrence 查询 |

后续要把 `EditorEditCommand`、selection state 和 controller orchestration 分开，避免 `common/commands` 反向依赖 `editorSelectionController` 形成循环。

### 2.4 `common/commands`

`commands` 是纯编辑操作的编排层。它可以调用 model/cursor contract，但不能访问键盘、鼠标、DOM 或 Workbench command registry。

| 目标文件 | 职责 |
| --- | --- |
| `common/commands/blockCommentCommands.ts` | block comment 的 edit plan |
| `common/commands/lineCommentCommands.ts` | line comment 的 edit plan |
| `common/commands/lineJoin.ts` | selected lines 的 join edit |
| `common/commands/selectionText.ts` | 按稳定 selection 顺序读取文本 |
| `common/commands/textSearchCommands.ts` | search replacement 和 isolated edit |
| `common/commands/clipboard.ts` | clipboard entry 的纯数据和 line/selection paste policy |
| `common/commands/gotoLocation.ts` | line/column/offset 输入解析和 model clamp |
| `common/commands/editorCommand.ts` | Current：`createEditorEditCommand` 将当前 selection 和 ordered text edits 映射为统一 command |

浏览器中的 `blockCommentController`、`findController`、`lineJoinController` 等只负责路由输入并调用这里的 command；controller 不应重新实现 edit 规则。

### 2.5 `common/viewLayout` 与 `common/viewModel`

Aster 已将原来的 `common/view` 拆成 `viewLayout` 与 `viewModel`；后续新增的纯布局/投影算法必须继续落在这两个目录，不得回建 `common/view`：

| 当前文件 | 目标文件 | 目标职责 |
| --- | --- | --- |
| `common/view/viewport.ts` | `common/viewLayout/editorViewportModel.ts` | scroll、visible line、overscan、layout change；Current |
| `common/view/visualLineProjection.ts` | `common/viewModel/modelLineProjection.ts` | logical line 到 visual line 的投影；Current |
| `browser/visualLineProjection.ts` | `browser/view/visualLineProjection.ts` | browser wrapping/measurement adapter |
| `browser/visibleLineProjection.ts` | `browser/view/visibleLineProjection.ts` | folding/hidden line 的可见行组合 |
| `browser/editorViewport.ts` | `browser/view/editorViewport.ts` | DOM viewport、render scheduling、hit testing 组合 |
| `browser/renderedLine.ts` | `browser/view/renderedLine.ts` | 一行的 DOM/render representation |
| `browser/rangeGeometry.ts` | `browser/view/rangeGeometry.ts` | model range 到像素矩形 |
| `browser/selectionGeometry.ts` | `browser/view/selectionGeometry.ts` | selection 到 presentation geometry |
| `browser/decorationPresentation.ts` | `browser/view/decorationPresentation.ts` | decoration snapshot 到 renderer presentation |

`viewLayout` 和 `viewModel` 只能读取 model snapshot/事件。DOM、font measurement、CSS token、accessibility state 和 pointer event 都属于 `browser/view`。

### 2.6 `common/services`

`common/services` 只放 Aster editor 自己的前端契约。它不能直接暴露 Workbench transport 或 generated Rust DTO。

| 目标文件 | 职责 | 当前状态 |
| --- | --- | --- |
| `common/services/textModelService.ts` | `ITextModelService`、model reference 和 conflict contract | Current，Workbench 仅通过 `browser/services` adapter 接入 |
| `common/services/textResourceStore.ts` | `ITextResourceStore`、resolve/save/change notification 的 editor-owned contract | Current |
| `common/services/diffComputationService.ts` | diff request/result 的 editor-owned contract | Current，位于 `common/diff` |
| `common/services/languageService.ts` | provider registry、per-model language services 和 feature factory | Current；Workbench 只保留 DI wrapper |

`BrowserTextResourceStore` 应位于 browser/runtime adapter 中，把 Workbench 的 `ITextFileService` 转换为 editor resource contract；`BrowserTextModelService` 负责 model cache、dirty、external change 和 save serialization。这样 `common/services` 不再依赖 Workbench。

### 2.7 `common/languages` 与 `common/tokens`

当前 `language/common` 是一个完整的 Aster language 子系统，但文件位置还没有对齐 VS Code editor 的 common language/tokens 分区。目标拆分如下：

| 当前范围 | 目标范围 | 说明 |
| --- | --- | --- |
| `language/common/languageResults.ts` | `common/languages/languageResults.ts` + `common/tokens/languageTokens.ts` | request/result/diagnostic 属于 language；token value 与 line index 属于 tokens |
| `language/common/languageResultStore.ts` | `common/languages/languageResultStore.ts` | 版本化结果 store |
| `language/common/languageRequestCoordinator.ts` | `common/languages/languageRequestCoordinator.ts` | cancellation、request identity、stale result gate |
| `language/common/syntax*` | `common/languages/syntax/*` | provider、wire、delta、snapshot normalizer；Current |
| `language/common/languageCompletion*` | `common/languages/completion/*` | completion provider/session/wire；Current |
| `language/common/languageTokenLineIndex.ts` | `common/tokens/languageTokenLineIndex.ts` | token line query和增量 index |
| `language/browser/*Worker*` | `browser/language/*Worker*` | browser Worker lifecycle 和 port adapter |
| `language/browser/semanticTokenPresentation.ts` | `browser/view/semanticTokenPresentation.ts` | token 到 CSS/presentation 的投影 |

语言 provider 的异步实现可以由 Worker 或 Rust adapter 提供，但 result store 必须在 Aster common 侧完成版本校验；过期结果不能重新写入当前 model。

### 2.8 `common/diff`

`common/diff` 只保留 editor-owned 的 request/result/domain types 和 diff model。实际计算位于 `browser/services/rustDiffComputationService.ts`，生产环境只允许 Rust App Server 路径。

| 文件 | 职责 |
| --- | --- |
| `common/diff/lineDiff.ts` | line hunk、row、range 的 0-based UTF-16 domain types |
| `common/diff/diffComputationService.ts` | `IDiffComputationService` contract |
| `common/diff/diffModel.ts` | original/modified model 与 diff result 的生命周期 |
| `browser/services/rustDiffComputationService.ts` | Rust protocol 到 Aster domain 的机械适配 |

## 3. `contrib` 的统一结构

每个 Aster contribution 都必须使用下面的目录形状；不因为当前只有一个文件就把它放回 `common` 根目录：

```text
contrib/<feature>/
  common/                 pure state, domain contract, edit/query algorithm
  browser/                controller, DOM presentation, editor contribution
  test/common/            common contract tests
  test/browser/           browser/controller tests
  browser/media/          feature-owned CSS only when needed
  <feature>.contribution.ts  optional complex assembly entry point
```

规则：

1. `common` 不读 DOM、不读 Workbench service、不读 Rust DTO。
2. `browser` 可以组合 `common`、`model`、`viewModel` 和 frontend service，但不能修改 model 的隐含事务规则。
3. 每个 contribution 自己拥有 command、controller、presentation 和生命周期；共享的纯算法才进入 `common`。
4. contribution 不得通过深层 CSS selector 改写别的 component 的内部状态。
5. 不新增目录 `index.ts` barrel；简单功能在 browser 主文件中注册，需要 `configure`、能力注入或多对象编排时才建立 `.contribution.ts`。
6. 任何需要跨宿主复用的能力必须先定义 frontend-owned contract，再提供 browser/Rust/Workbench adapter。

## 4. VS Code `contrib` 全量迁移目录

下面的清单覆盖 VS Code 当前 `src/vs/editor/contrib` 目录，而不是只列 Aster 当前已经实现的功能。目标文件名尽量保持 VS Code 语义；`Aster` 前缀只用于明确的产品 presentation 或 runtime adapter。

### 4.1 编辑输入、选择和文本操作

| VS Code contrib | Aster 目标文件 | 当前 Aster 映射 | 状态 | 职责边界 |
| --- | --- | --- | --- | --- |
| `anchorSelect` | `contrib/anchorSelect/browser/anchorSelectController.ts` | `contrib/anchorSelect/browser/anchorSelectController.ts` | Current | 锚点选择和扩展范围，不进入 model |
| `caretOperations` | `common/cursor/caretOperations.ts` | `common/cursor/caretOperations.ts`、`cursorNavigation.ts`、`cursorInsertion.ts` | Current | caret 相邻位置、行边界和可见位置计算 |
| `cursorUndo` | `contrib/cursorUndo/browser/cursorUndoController.ts` | 同名 contrib | Current | editor instance selection-only undo；文本 undo 仍归 model |
| `multicursor` | `contrib/multicursor/common/multiCursorOperations.ts`、`browser/multiCursorController.ts` | `common/cursor/*`、`browser/multiCursorController.ts` | Current | 多光标增删、合并、主 selection 规则 |
| `wordOperations` | `common/cursor/wordOperations.ts`、`contrib/wordOperations/browser/wordOperationsController.ts` | `common/cursor/wordOperations.ts`、`cursorWordOperations.ts`、`wordBoundary.ts` | Current | word 删除、选择和跨 selection 边界 |
| `wordPartOperations` | `common/cursor/wordPartOperations.ts` | `common/cursor/wordPartOperations.ts`、`cursorWordOperations.ts`、`core/wordHelper.ts` | Current | camelCase、subword 和语言无关 word part |
| `lineSelection` | `contrib/lineSelection/browser/lineSelection.ts` | `contrib/lineSelection/browser/lineSelection.ts` | Current | 物理行选择扩展 |
| `linesOperations` | `contrib/linesOperations/browser/linesOperations.ts`、`lineOperationsController.ts` | 同名 contrib | Current | indent、duplicate、move、delete、copy line groups |
| `indentation` | `common/editorIndentation.ts`、`browser/view/indentationGuides.ts` | engine configuration + viewport projection | Current（职责归位） | indentation calculation 与 guides presentation 是基础 engine contract；line indentation command 仍在 contrib |
| `comment` | `contrib/comment/common/{blockCommentCommands,lineCommentCommands}.ts`、browser controllers | 同名 contrib | Current | comment language contract、toggle command 和 selection presentation |
| `clipboard` | `contrib/clipboard/common/clipboard.ts`、browser clipboard 系列 | 同名 contrib | Current | copy/cut/paste、selection metadata、safe HTML 和 line policy |
| pointer selection | `browser/input/{pointerSelectionController,pointerAutoScroll,pointerMultiCursor}.ts` | `CodeEditorWidget` | Current | 基础 pointer selection、拖动自动滚动与修饰键多光标；不是 contrib，也不处理外部 drop |
| `dropOrPasteInto` | `contrib/dropOrPasteInto/browser/{textDropController,textFileTransfer}.ts` | 同名 contrib，由完整 `EditorPart` 装配 | Current | file/text/URI drop、MIME provider 和 paste provider；基础 `CodeEditorWidget` 不硬编码该可选行为 |
| `insertFinalNewLine` | `contrib/insertFinalNewLine/{common/insertFinalNewLine.ts,browser/insertFinalNewLineController.ts}` | 同名 contrib | Current | save 边界的最终换行策略，不由 TextModel 隐式执行 |
| `inPlaceReplace` | `contrib/inPlaceReplace/browser/inPlaceReplaceController.ts` | `contrib/inPlaceReplace/browser/inPlaceReplaceController.ts` | Current | 当前 selection 的 next/previous replacement |
| `smartSelect` | `contrib/smartSelect/common/smartSelect.ts`、`browser/smartSelectController.ts` | `contrib/smartSelect/common/smartSelect.ts`、`browser/smartSelectController.ts` | Current | 语法/词边界扩展选择；不能把语言 AST 直接塞进 model |
| `snippet` | `contrib/snippet/common/{snippetParser,snippetSession,snippetTransform}.ts` | 同名 contrib | Current | snippet parse、placeholder、transform 和 linked selection |

### 4.2 查找、语言和代码理解

| VS Code contrib | Aster 目标文件 | 当前 Aster 映射 | 状态 | 职责边界 |
| --- | --- | --- | --- | --- |
| `bracketMatching` | `contrib/bracketMatching/common/*`、`browser/*` | 同名 contrib | Current | lexical bracket matching、pair editing、enter、colorization 和 marker presentation |
| `folding` | `contrib/folding/browser/foldingModel.ts`、`foldingRanges.ts`、`hiddenRangeModel.ts` | 同名 contrib | Current | ranges、hidden lines、tracked fold state 和 gutter |
| `semanticTokens` | `contrib/semanticTokens/common/semanticTokens.ts`、`browser/semanticTokenPresentation.ts` | `contrib/semanticTokens/common/semanticTokens.ts`、`browser/semanticTokenPresentation.ts` | Current | token result、line index、version gate 和 renderer projection |
| `tokenization` | `contrib/tokenization/common/tokenizationTextModelPart.ts`、`browser/tokenizationController.ts` | `contrib/tokenization/{common,browser}/*`、Aster syntax/TextMate adapters | Current | token production、line state 和 token invalidation |
| `wordHighlighter` | `contrib/wordHighlighter/common/wordHighlighter.ts`、`browser/wordHighlighterController.ts` | 同名 contrib | Current | word occurrence query、decoration owner 和 presentation |
| `find` | `contrib/find/browser/findController.ts`、`common/model/textModelSearch.ts` | 同名 contrib + model search | Current | search model、find widget、replace、selection scope |
| `gotoError` | `contrib/gotoError/common/diagnosticDecorations.ts`、`browser/gotoError.ts`、`browser/languageDiagnosticPresentation.ts` + `browser/view/diagnosticOverviewMarkers.ts` | 同名 contrib + viewport aggregation | Current | diagnostic decoration、navigation、overview、severity ordering |
| `gotoSymbol` | `contrib/gotoSymbol/common/gotoSymbol.ts` | `contrib/gotoSymbol/{common,browser}/*`、Aster language service factory | Current | provider-backed symbol navigation；不与 goto line 混合 |
| `documentSymbols` | `contrib/documentSymbols/common/documentSymbols.ts` | `contrib/documentSymbols/common/documentSymbols.ts`、Aster language service factory | Current | versioned document symbol result、outline/quick navigation contract |
| `hover` | `contrib/hover/{common/hover.ts,browser/{hoverController,diagnosticHoverController}.ts}` | 同名 contrib | Current | 通用 hover provider、diagnostic hover、anchor、content widget |
| `links` | `contrib/links/browser/linksController.ts` | `contrib/links/{common,browser}/*` | Current | link detection、hover、open action；外部打开交给 host |
| `codeAction` | `contrib/codeAction/common/codeAction.ts` | `contrib/codeAction/{common,browser}/*`、Aster language service factory | Current | provider result、resolve、apply edit |
| `format` | `contrib/format/{common/formatCommands.ts,browser/formatController.ts}` | 同名 contrib | Current | format/range/on-type format 的 edit application |
| `rename` | `contrib/rename/{common/rename.ts,browser/renameController.ts}` | 同名 contrib | Current | rename input、provider freshness、workspace edit |
| `inlayHints` | `contrib/inlayHints/common/inlayHints.ts` | `contrib/inlayHints/{common,browser}/*`、Aster language service factory | Current | versioned inline hint result和view projection contract |
| `inlineCompletions` | `contrib/inlineCompletions/common/inlineCompletions.ts` | `contrib/inlineCompletions/{common,browser}/*`、Aster language service factory | Current | ghost text、request freshness、accept/reject contract |
| `parameterHints` | `contrib/parameterHints/common/parameterHints.ts` | `contrib/parameterHints/{common,browser}/*`、Aster language service factory | Current | signature help 的 request/session/widget contract |
| `suggest` | `contrib/suggest/{common/suggestModel.ts,browser/suggestWidget.ts}` | 同名 contrib | Current | trigger、filter、resolve、accept、incomplete refresh |
| `linkedEditing` | `contrib/linkedEditing/common/linkedEditing.ts` | `contrib/linkedEditing/{common,browser}/*`、Aster language service factory | Current | linked ranges 和同步 edit；必须使用 model transaction |
| `colorPicker` | `contrib/colorPicker/common/color.ts`、`browser/colorPickerController.ts` | `contrib/colorPicker/{common,browser}/*` | Current | color range provider 与 editor color widget |
| `codelens` | `contrib/codelens/common/codelens.ts` | `contrib/codelens/{common,browser}/*`、Aster language service factory | Current | versioned code lens、resolve 和 inline presentation |

### 4.3 视图、导航和编辑器辅助 UI

| VS Code contrib | Aster 目标文件 | 当前 Aster 映射 | 状态 | 职责边界 |
| --- | --- | --- | --- | --- |
| `editorState` | `contrib/editorState/common/editorState.ts`、`browser/editorStateController.ts` | `contrib/editorState/{common,browser}/*`、part 装配 | Current | editor focus、model、selection、scroll 的可观察状态 |
| `contextmenu` | `contrib/contextmenu/browser/contextMenuController.ts` | `contrib/contextmenu/browser/contextMenuController.ts`；host callback 可选 | Partial | context menu action 组合；不定义 command 语义 |
| `diffEditorBreadcrumbs` | `contrib/diffEditorBreadcrumbs/browser/diffEditorBreadcrumbs.ts` | Rust diff model + `DiffEditorPane` 装配 | Current | diff editor 当前 hunk/文件导航，不参与 diff 计算 |
| `floatingMenu` | `contrib/floatingMenu/browser/floatingMenuController.ts` | `contrib/floatingMenu/browser/floatingMenuController.ts`；宿主按 action 注入 | Current | selection/hover anchor 的 transient menu |
| `fontZoom` | `contrib/fontZoom/browser/fontZoomController.ts` | `contrib/fontZoom/browser/fontZoomController.ts` + `EditorViewport.refreshFontMetrics` | Current | editor font zoom state 与 measurement invalidation |
| `gpu` | `browser/view/gpuMinimapRenderer.ts` | viewport minimap | Current（职责归位） | GPU 是 viewport 的可降级实现细节；不得让 viewModel 依赖 WebGL |
| `longLinesHelper` | `browser/view/lineWidthIndex.ts` | viewport budgets | Current（职责归位） | line measurement 是 viewport 基础算法，不伪装成可卸载 command contrib |
| `middleScroll` | `contrib/middleScroll/browser/middleScrollController.ts` | `contrib/middleScroll/browser/middleScrollController.ts` | Current | middle-button scroll，不污染普通 pointer selection |
| `quickAccess` | `contrib/quickAccess/browser/quickAccessController.ts` | 同名 contrib；当前实现 Go to Line/Column | Current | editor-local quick access；Workbench global quick open 不归 Aster |
| `peekView` | `contrib/peekView/browser/peekViewWidget.ts` | `contrib/peekView/browser/peekViewWidget.ts`；宿主按需创建 | Current | anchored preview surface 和生命周期 |
| `placeholderText` | `contrib/placeholderText/browser/placeholderTextController.ts` | `contrib/placeholderText/browser/placeholderTextController.ts`；part 可选装配 | Current | empty model 的 presentation placeholder |
| `readOnlyMessage` | `contrib/readOnlyMessage/browser/readOnlyMessageController.ts` | 同名 contrib | Current | 用户可见的 readonly feedback，不参与权限判定 |
| `sectionHeaders` | `contrib/sectionHeaders/browser/sectionHeadersController.ts` | `contrib/sectionHeaders/browser/sectionHeadersController.ts` | Current | folding/outline section header presentation |
| `stickyScroll` | `contrib/stickyScroll/common/stickyScrollModel.ts`、`browser/stickyScrollController.ts` | `contrib/stickyScroll/{common,browser}/*` | Current | visible hierarchy 的 sticky projection |
| `symbolIcons` | `contrib/symbolIcons/browser/symbolIconsController.ts` | `contrib/symbolIcons/browser/symbolIconsController.ts` | Current | symbol kind 到 icon presentation |
| `toggleTabFocusMode` | `contrib/toggleTabFocusMode/browser/toggleTabFocusModeController.ts` | `contrib/toggleTabFocusMode/browser/toggleTabFocusModeController.ts` | Current | Tab focus mode 的 input routing，不改变 model |
| `unicodeHighlighter` | `contrib/unicodeHighlighter/common/unicodeHighlighter.ts`、`browser/unicodeHighlighterController.ts` | `contrib/unicodeHighlighter/{common,browser}/*`；part 默认装配 | Current | confusable/invisible Unicode decoration |
| `unusualLineTerminators` | `contrib/unusualLineTerminators/common/unusualLineTerminators.ts`、`browser/unusualLineTerminatorsController.ts` | `contrib/unusualLineTerminators/{common,browser}/*` | Current | 诊断原始 line terminator，不改变 model contract |
| `message` | `contrib/message/browser/messageController.ts` | `contrib/message/browser/messageController.ts` | Current | editor-local transient message，不替代 Workbench notification |
| `inlineProgress` | `contrib/inlineProgress/browser/inlineProgressController.ts` | `contrib/inlineProgress/browser/inlineProgressController.ts` | Current | provider request 的 inline progress presentation |
| `zoneWidget` | `contrib/zoneWidget/browser/zoneWidget.ts` | `contrib/zoneWidget/browser/zoneWidget.ts`；宿主按需创建 | Current | model range 附近的可交互 widget 容器 |

## 5. Aster 当前文件到目标文件的迁移表

| 当前路径 | 目标路径 | 迁移动作 | 状态 |
| --- | --- | --- | --- |
| `common/view/viewport.ts` | `common/viewLayout/editorViewportModel.ts` | 重命名并保持无 DOM | Current |
| `common/view/visualLineProjection.ts` | `common/viewModel/modelLineProjection.ts` | 拆出 model projection | Current |
| `browser/editorViewport.ts` | `browser/view/editorViewport.ts` | 只保留 DOM viewport 组合 | Current |
| `browser/renderedLine.ts` | `browser/view/renderedLine.ts` | 归入 view renderer | Current |
| `browser/visualLineProjection.ts` | `browser/view/visualLineProjection.ts` | 归入 browser view | Current |
| `browser/textInputController.ts` | `browser/input/textInputController.ts` | 输入 adapter，不改 common command | Current |
| `browser/compositionController.ts` | `browser/input/compositionController.ts` | DOM composition adapter | Current |
| `browser/clipboardController.ts` | `contrib/clipboard/browser/clipboardController.ts` | contribution 化 | Current |
| `browser/textDropController.ts` | `contrib/dropOrPasteInto/browser/textDropController.ts` | contribution 化 | Current |
| `browser/findController.ts` | `contrib/find/browser/findController.ts` | contribution 化 | Current |
| `browser/cursorUndoController.ts` | `contrib/cursorUndo/browser/cursorUndoController.ts` | contribution 化 | Current |
| `browser/blockCommentController.ts` | `contrib/comment/browser/blockCommentController.ts` | comment command routing | Current |
| `browser/lineCommentController.ts` | `contrib/comment/browser/lineCommentController.ts` | command routing | Current |
| `browser/lineJoinController.ts` | `contrib/linesOperations/browser/lineJoinController.ts` | line operation contribution | Current |
| `browser/occurrenceHighlightController.ts` | `contrib/wordHighlighter/browser/wordHighlighterController.ts` | occurrence presentation迁移 | Current |
| `browser/occurrenceSelectionController.ts` | `contrib/multicursor/browser/occurrenceSelectionController.ts` | selection feature迁移 | Current |
| `language/common/*` | `common/languages/*`、`common/tokens/*` | syntax/completion/token拆分 | Current |
| `language/browser/*` | `browser/language/*` | Worker 和浏览器 adapter 归位 | Current |
| `browser/diff/rustDiffComputationService.ts` | `browser/services/rustDiffComputationService.ts` | runtime adapter 归服务层 | Current |
| `browser/browserTextModelService.ts` | `browser/services/browserTextModelService.ts` | model reference、dirty/conflict 与保存语义归 Editor；文件 I/O 通过 `ITextResourceStore` 注入 | Current |

### 5.1 当前实现台账：按文件阅读 Aster 的入口

下面是实现代码的 canonical 阅读顺序。表中的“入口”是应该首先打开的文件；同一目录下的辅助文件不能绕过入口自行定义第二套语义。

| 层 | 入口文件 | 文件职责与允许依赖 | 当前维护规则 |
| --- | --- | --- | --- |
| core | `common/core/{position,range,selection,textChange,editOperation}.ts` | 0-based 行、UTF-16 列、end-exclusive range、selection 方向、文本变化和原子 edit 值对象；只依赖 `base/common` | 不放 model、DOM、语言 provider 或 Rust DTO |
| core/text | `common/core/text/{abstractText,textLength,positionToOffsetImpl,getPositionOffsetTransformerFromTextModel}.ts` | 文本长度、position/offset 变换和纯文本抽象；transformer 接受 `TextModelLineSource`，core 不导入具体 model | 所有 offset 必须是 UTF-16；不得引入 1-based 兼容语义 |
| core/edits | `common/core/edits/*.ts` | array、line、length、string、text edit 的纯算法 | 算法不读取 history 或 selection controller |
| core/ranges | `common/core/ranges/*.ts` | line/column/offset/range mapping | 不读取 DOM 坐标 |
| core/misc | `common/core/misc/{eolCounter,indentation,rgba,textModelDefaults}.ts` | 通用 EOL、缩进、颜色和文本默认值 | 不新增目录 barrel；使用直接文件路径 |
| model | `common/model/textModel.ts` | Piece Tree 之上的同步文档权威：文本、版本、事务、snapshot、undo/redo、change event | 所有输入 edit 先在这里一次性提交；不得等待异步服务 |
| model/storage | `common/model/pieceTreeTextBuffer/{pieceTreeTextBuffer,pieceTreeBase,pieceTreeSnapshot}.ts` | Piece Tree、节点统计、snapshot segment 读取 | 存储优化不能改变 model 的 version/change contract |
| model/history | `common/model/{editStack,historyCoalescing}.ts` | 事务级历史、输入合并、undo/redo budget | 光标历史不放这里，光标历史属于 cursor |
| model/query | `common/model/textModelSearch.ts` | literal、regex、whole-word 和 wrap search | 结果必须绑定当前 model version |
| model/range | `common/model/{trackedRange,decoration}.ts` | tracked range stickiness、decoration collection 和 model change 映射 | 不解释 CSS，不知道语言 severity |
| cursor | `common/cursor/editorSelectionController.ts` | editor instance 的 selection、composition、cursor-only history 和 `EditorEditCommand` 执行 | 不注册 DOM listener 或 Workbench command |
| cursor/operations | `common/cursor/cursor*.ts`、`editorSelectionOperations.ts`、`wordBoundary.ts`、`columnSelection.ts` | 导航、插入、删除、word/overtype/transpose、column selection 和 selection offset 映射 | 操作只返回 command；不得直接写 DOM |
| commands | `common/commands/{editorCommand,editorEditCommand,gotoLocation,selectionText,textSearchCommands}.ts` | command contract、edit/selection 映射、位置解析和搜索替换 | command 不知道键盘、鼠标或 Workbench |
| viewLayout | `common/viewLayout/editorViewportModel.ts` | scroll、visible logical lines、overscan、layout change | 只消费 line source；不引入 DOM |
| viewModel | `common/viewModel/modelLineProjection.ts` | logical line 到 visual line 的纯投影 | folding 通过输入 contract 注入，不能依赖 folding controller |
| languages/base | `common/languages/{languageConfiguration,languageId,languageLexical*,languageProviderModules}.ts` | language ID/configuration、词法上下文、provider module 生命周期 | 只保留语言无关的 common contract |
| languages/syntax | `common/languages/syntax/*.ts` | token/diagnostic provider registry、worker wire、request coordinator、delta 和 result freshness | result 先经过版本 gate，再交给 tokens/diagnostics |
| languages/completion | `common/languages/completion/*.ts` | completion provider、catalog/resolve wire、completion result 和 word provider | snippet 实现位于 `contrib/snippet/common`，completion 只消费 parser contract |
| languages/results | `common/languages/{languageResults,languageResultStore,languageRequestCoordinator}.ts` | diagnostic 结果、版本化 store、request identity/stale result gate | token value 已从这里迁到 `common/tokens` |
| tokens | `common/tokens/{languageTokens,languageTokenLineIndex}.ts` | token value、delta、normalizer、按行索引 | 不渲染 CSS；presentation 由 browser/view 负责 |
| services | `common/services/{languageService,textModelService,textResourceStore}.ts` | Aster-owned provider factory、model reference、resource resolve/save/change contract | 不导出 Workbench transport 或 generated DTO |
| browser/view | `browser/view/*.ts` | DOM viewport、rendered line、geometry、minimap、semantic token 和 decoration presentation | view 只能读取 model/viewModel snapshot；CSS 由 view owner 管理 |
| browser/input | `browser/input/{textInputController,compositionController,keyboardNavigationController,pointerSelectionController,pointerAutoScroll,pointerMultiCursor}.ts` | textarea、IME、键盘和 pointer navigation、输入事件适配 | 输入热路径不等待 IPC/RPC |
| editor browser/services | `browser/services/{browserTextModelService,rustDiffComputationService,rustSyntaxFactsService}.ts` | 管理 model reference/dirty/conflict，并将前端 `IDiffApi` / `ISyntaxApi` 结果投影为 editor facts | 不引用 Workbench、IPC 或 generated App Server DTO；强制能力缺失时显式报错 |
| Workbench adapter | `workbench/contrib/codeEditor/browser/{browserTextResourceStore,browserEditorPart}.ts` | 文件、TextMate、worker 与产品服务接线 | 只实现 editor-owned contract，不复制 model、selection 或 contribution 行为 |
| contrib/editing | `contrib/{clipboard,comment,dropOrPasteInto,linesOperations,transpose,wordWrap,insertFinalNewLine}/` | 每个功能自己拥有 common command、browser controller、presentation 和 tests | 新增功能不得回填 browser 根目录 |
| contrib/language UX | `contrib/{bracketMatching,folding,gotoError,hover,suggest,snippet,format,rename}/` | 语言驱动的编辑、诊断、hover、completion、format 和 rename | provider contract 在 common；DOM/widget 在 browser |
| contrib/query contracts | `contrib/{documentSymbols,gotoSymbol,links,codeAction,inlayHints,inlineCompletions,parameterHints,linkedEditing,codelens}/common/` | provider request/result、版本 freshness、resolve 和 edit contract | 没有浏览器 UI 时仍必须保持可测试的 common contract；不得创建空 controller |

### 5.1.1 Rust syntax facts and frontend syntax runtime

`syntax` is deliberately not a generic language-processing subsystem. It is the
runtime boundary for parser/grammar-derived token and diagnostic facts. Its
producer, synchronization, and presentation owners stay separate:

| Responsibility | Canonical owner | Contract | Must not own |
| --- | --- | --- | --- |
| Parser-derived tokens, diagnostics, symbols, and fold ranges | `zeta-rs/app-server` `syntax/analyze` | bounded revision-bound UTF-16 DTOs | DOM, editor state, decorations, or worker lifecycle |
| Snapshot synchronization, provider priority/fallback, result freshness, and wire deltas | Aster `common/languages/syntax/*` | `SyntaxService`, `SyntaxWorker`, `SyntaxProvider` | parser implementation, App Server transport, or product file access |
| TextMate grammar tokenization | `workbench/services/textMate` | `TextMateSyntaxWorker` contributes a high-priority `SyntaxProvider` | document model, diagnostics UI, or App Server syntax facts |
| Token spans, markers, symbol navigation, and folding presentation | Aster `common/tokens` plus the matching `contrib` | versioned stores consumed by the respective contrib | parser state or cross-editor part state |
| Structured-document editing | Aster | `textBlock` may use the line editor only through editor-owned `IEmbeddedTextEditorFactory` | line-editor syntax runtime, TextMate worker, or code-editor part state |

The Rust adapter is optional and bounded. Unsupported languages and oversized
documents fall back to the frontend provider chain; a failed Rust request does
not make the token or diagnostic stores publish a stale result. This keeps
interactive UI work in Aster while moving parser-grade, backend-neutral facts
to `zeta-rs`.

### 5.2 当前已装配的 Aster editor part

`browser/editorPart.ts` 是单个 editor instance 的装配入口，顺序固定为：

```text
TextModelReference
  → EditorSelectionController
  → folding/hidden ranges
  → syntax/tokens/diagnostics
  → completion/suggest/snippet
  → CodeEditorWidget(view + input)
  → bracket/comment/lines/find/quickAccess/hover/format/rename/readOnly/save
```

`workbench/contrib/codeEditor/browser/browserEditorPart.ts` 提供 TextMate grammar readiness、syntax Worker 和 completion Worker；它只能通过 editor-owned options/contract 注入能力。`browser/editorContribution.ts` 提供 feature-neutral 静态注册与 typed capability lookup，不能 import 具体 contrib。`contrib/codeEditorPart.contribution.ts` 只建立 line-editor engine runtime 与共享 capability map；简单 controller 由功能主文件注册，复杂装配才保留独立 `.contribution.ts`。`workbench/contrib/codeEditor/browser/codeEditor.contribution.ts` 负责 pane 注册和强制 adapter 注入，Code Pane 还拥有 `Ctrl/Cmd+S` 保存命令。根级 `editor.all.ts` 是共同验证的标准 profile，`editor.code.all.ts` 与 `editor.academic.all.ts` 只能追加产品差异；不承诺任意 contribution 子集都是受支持组合。新 contribution 若需要共享 editor runtime 对象，应新增窄 `TextEditorCapability<T>`；若需要宿主能力，则先扩展 editor-facing callback/service contract，再由 Workbench adapter 注入。

### 5.3 当前仍保留的宿主边界

本节只记录真实未完成的产品接入，不把已经落地的 Aster controller 再写成 Planned。详细文件职责和装配情况见实现台账：

| 能力 | Aster 当前实现 | 仍由宿主决定的部分 |
| --- | --- | --- |
| contextmenu | hit-test、selection、model version 和 request contract | Workbench 菜单 action、菜单服务和右键后的产品命令集合 |
| links | provider link detection、hover target 和生命周期 | 外部 URI 打开、权限和工作区安全策略 |
| codeAction / codelens | provider result、resolve、inline presentation 和 edit/command contract | picker 的产品 action、命令注册和 workspace command 执行 |
| floatingMenu / peekView / zoneWidget | 可复用的 editor-local surface、定位和 dispose 生命周期 | 具体 action、内容来源、持久化和 pane/workbench 组合 |
| Rust-backed language/file capabilities | Aster frontend contract 和 browser adapter | App Server transport、权限、超时和错误上报策略 |

这些边界不阻塞 Aster 编辑器本身；它们是明确的 host callback 或 adapter contract，而不是空目录或 fallback。

## 6. 迁移顺序

### Phase 0：冻结设计契约

Current：本文、`README.md` 和 `docs/editor-architecture.md` 共同描述 Aster 当前所有权。

完成标准：

- 所有新文件都能在本文找到 layer owner。
- 所有 `contrib` 都明确 `common`、`browser`、`test` 的职责。
- `Current`、`Partial`、`Planned`、`Non-goal` 不混写。
- 文档中的目标文件名不会被误读成当前已实现文件。

### Phase 1：纯化 core/model（Current）

1. 修正 core 反向依赖 model 的 transformer。Current：使用 `TextModelLineSource`，具体 model 只在 adapter 侧提供。
2. 将 model 内部的 event、history、tracked range 依赖保持在 model/base。
3. 为 `TextModel`、`TextDecorationCollection`、`TrackedRange` 固定 public contract 和 failure semantics。
4. 保持 0-based UTF-16、LF、end-exclusive 和同步 transaction 不变。

### Phase 2：完成 viewLayout/viewModel（Current，browser projection 持续演进）

1. 将 `common/view` 拆为 `viewLayout` 与 `viewModel`。Current：两个目录已存在，旧目录无实现文件。
2. 从 browser viewport 中移出纯 scroll/line projection 算法。
3. browser 只消费 snapshot、projection 和 measured geometry。
4. folding 的 hidden-line projection 通过 viewModel contract 注入，不让 viewModel 依赖 folding implementation。

### Phase 3：完成 services/adapters（Current）

1. 新增 `ITextResourceStore`。Current：`common/services/textResourceStore.ts` + `workbench/contrib/codeEditor/browser/browserTextResourceStore.ts`。
2. `BrowserTextResourceStore` 适配 Workbench `ITextFileService`。
3. `BrowserTextModelService` 只负责 model cache、baseline、dirty 和 conflict。
4. Rust diff/language/file adapters 只实现 Aster-owned frontend contract。
5. 删除 production fallback；缺少强制 transport 时显式失败。

### Phase 4：迁移已有 contrib（Current，新增能力继续按同一规则进入）

优先迁移已经存在而且边界清晰的功能：

1. `folding`
2. `indentation`
3. `lineSelection`
4. `linesOperations`
5. `clipboard`
6. `dropOrPasteInto`
7. `find`
8. `cursorUndo`
9. `comment`
10. `wordHighlighter`
11. `multicursor`
12. `semanticTokens`、`tokenization`

每次迁移只做 path/ownership 变化，不同时改变用户行为；行为变化必须有独立测试和文档状态。

### Phase 5：补齐语言与高级 Editor UX（Current；宿主边界持续接入）

优先级：

- P0：bracketMatching、suggest、snippet、gotoError、readOnlyMessage。
- P1：hover、documentSymbols、gotoSymbol、links、format、rename、codeAction、inlayHints。common contract、browser controller 和 part 装配已完成；外部打开/产品 action 通过 host callback 保持边界。
- P2：inlineCompletions、parameterHints、linkedEditing、codelens、colorPicker。common contract、browser controller 和 part 装配已完成。
- P3：peekView、stickyScroll、sectionHeaders、quickAccess、zoneWidget、floatingMenu、fontZoom、unicodeHighlighter。editor-local surface/controller 已完成；宿主按需注入具体内容或 action。

P2/P3 功能必须先有稳定的 language/service contract；不能为了填满 contrib 目录而建立空壳 controller。

### Phase 6：依赖约束和验收

增加 Aster 专用依赖检查，至少阻止：

```text
common/core      → common/model
common/model     → common/services / browser / language
common           → workbench / electron / generated app-server DTO
viewModel        → DOM / CSS / browser event
contrib/common   → browser / Workbench
```

每个迁移切片必须通过：

- `pnpm typecheck:renderer`
- Aster common/browser focused tests
- contribution common/browser tests
- `git diff --check`
- source path stale-reference scan

## 7. 文件命名和提交规则

### 7.1 命名

- VS Code 有明确语义的 feature，优先使用同名目录：`folding`、`find`、`semanticTokens`、`wordHighlighter`。
- Editor common contract 使用领域名，不使用 `Aster` 前缀；例如 `TextModel`、`EditorViewportModel`、`LanguageTokenResult`。
- Browser 产品 presentation 或 adapter 只在运行时语义确实需要时使用 `Browser` 前缀；不再使用表示开发阶段的 `Aster` 前缀，例如 `BrowserTextResourceStore`。
- Rust adapter 文件必须显式包含 `Rust` 或 `AppServer`，避免把 transport 名称隐藏在 generic service 中。
- 不新增 `index.ts` barrel；使用明确 import path。

### 7.2 一个功能的最小文件集合

```text
contrib/find/
  common/findModel.ts
  common/findCommands.ts
  browser/findController.ts
  browser/findWidget.ts
  test/common/findModel.test.ts
  test/browser/findController.test.ts
  find.contribution.ts
```

`common` 文件不应 import `findController`；`findController` 不应实现 regex/search semantics；`find.contribution.ts` 只负责注册和装配。

### 7.3 修改影响

修改 `model` 时必须检查 transaction、version、history、tracked range、decoration 和 language snapshot；修改 `viewModel` 时必须检查 projection、folding、viewport 和 hit-test；修改 contrib 时必须检查对应 command、controller、presentation、registration 和 test。

## 8. 不迁移到 Aster editor 的内容

下列内容不属于 Aster editor common，也不因为 VS Code 有对应目录就搬进来：

- Workbench tab、group、explorer、chat、session 和 workspace orchestration。
- Electron main/preload、Node 文件系统和 App Server generated DTO。
- Rust synchronous document core；Native editor 有自己的 Rust ownership。
- 通用 base primitive 的 editor-specific identity、selection、decoration 和 model version。
- 仅为未来功能建立的空目录、空 contribution 或未验证的 service singleton。

## 9. 当前验收标准

Aster editor 达到目标架构前，必须同时满足：

| 验收项 | 标准 |
| --- | --- |
| 行为 | 编辑、selection、undo、IME、folding、diff 不因文件迁移改变语义 |
| 依赖 | common/editor 层不反向依赖 Workbench、DOM 或 Rust DTO |
| 运行时 | Renderer 输入热路径无 IPC/RPC 阻塞，无 production fallback |
| 可替换性 | Rust、Worker、Workbench 文件服务都通过 Aster contract 接入 |
| 可测试性 | common 算法不需要 DOM；browser controller 有独立 JSDOM/集成测试 |
| 文档 | 每个 Current/Partial/Planned 状态都能指向真实文件或明确迁移任务 |
| 文件结构 | feature 文件名、目录名和 VS Code editor 语义保持可搜索的一致性 |

本文与实现不一致时，先更新本文的状态和 canonical owner，再修改代码；不能以历史路径作为新职责的依据。
