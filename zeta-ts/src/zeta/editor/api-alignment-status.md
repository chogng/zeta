# Editor API 对齐状态

> 本表记录 2026-08-30 对 `zeta-ts/src/zeta/editor` 生产 TypeScript 文件的扫描结果。分层依据为 VS Code 的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 和仓库内 `vscode-api-alignment` skill。

## 当前结论

- 已扫描 491 个本地生产文件，并与 661 个 VS Code Editor 生产文件比较。
- 初始确认 118 组同名声明结构差异；已处理 28 组，剩余 90 组，分布在 81 个文件中。
- 当前没有已知的 import owner 路径错误、全局唯一 owner 冲突或同 basename 放错路径候选。
- import 集合不同不单独判错：缺少上游能力会自然缺少对应 import，本地真实扩展也会增加 import。只有同一符号从错误 owner 导入才属于路径错误。
- 输入、参数和返回类型一致不代表行为已经一致。剩余项仍需继续核对状态 owner、事件顺序、失效条件、调度阶段、坐标转换、可见副作用、失败语义和释放时机。

## 处理规则

1. 同名、同职责且本地能力完整时，恢复上游名称、参数、返回值、owner 和调用链。
2. 本地职责与上游同名声明不同，且它确实是 Zeta 专有能力时，移出上游 owner 或改成明确的本地名称；不保留别名、包装入口或重复导出。
3. 缺少基础模型、视图上下文或服务契约时，从基础 owner 向调用方逐层实现，不在下游伪造同名接口。
4. `editor` 可以依赖 `base` 和 `platform`；`common` 不能依赖 DOM，`browser` 才能使用 DOM，依赖方向不能反转。

## 尚未补齐的同名契约

| 文件 | 声明 | 分类 |
| --- | --- | --- |
| `browser/controller/editContext/editContext.ts` | `AbstractEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/native/nativeEditContext.ts` | `NativeEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/native/nativeEditContextUtils.ts` | `FocusTracker` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderContentRich.ts` | `RichScreenReaderContent` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderContentSimple.ts` | `SimpleScreenReaderContent` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderSupport.ts` | `ScreenReaderSupport` | 输入与无障碍链路 |
| `browser/controller/editContext/textArea/textAreaEditContext.ts` | `TextAreaEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/textArea/textAreaEditContextInput.ts` | `TextAreaInput` | 输入与无障碍链路 |
| `browser/controller/mouseHandler.ts` | `MouseHandler` | 浏览器编辑器契约 |
| `browser/observableCodeEditor.ts` | `observableCodeEditor`、`ObservableCodeEditor` | 浏览器编辑器契约 |
| `browser/services/abstractCodeEditorService.ts` | `AbstractCodeEditorService` | 浏览器编辑器契约 |
| `browser/services/codeEditorService.ts` | `ICodeEditorOpenHandler`、`ICodeEditorService` | 浏览器编辑器契约 |
| `browser/stableEditorScroll.ts` | `StableEditorScrollState`、`StableEditorBottomScrollState` | 浏览器编辑器契约 |
| `browser/view/viewController.ts` | `ViewController` | 视图与布局链路 |
| `browser/view/viewUserInputEvents.ts` | `ViewUserInputEvents` | 视图与布局链路 |
| `browser/view.ts` | `View` | 视图与布局链路 |
| `browser/viewParts/blockDecorations/blockDecorations.ts` | `BlockDecorations` | 视图与布局链路 |
| `browser/viewParts/contentWidgets/contentWidgets.ts` | `ViewContentWidgets` | 视图与布局链路 |
| `browser/viewParts/currentLineHighlight/currentLineHighlight.ts` | `CurrentLineHighlightOverlay` | 视图与布局链路 |
| `browser/viewParts/decorations/decorations.ts` | `DecorationsOverlay` | 视图与布局链路 |
| `browser/viewParts/editorScrollbar/editorScrollbar.ts` | `EditorScrollbar` | 视图与布局链路 |
| `browser/viewParts/glyphMargin/glyphMargin.ts` | `GlyphMarginWidgets` | 视图与布局链路 |
| `browser/viewParts/indentGuides/indentGuides.ts` | `IndentGuidesOverlay` | 视图与布局链路 |
| `browser/viewParts/lineNumbers/lineNumbers.ts` | `LineNumbersOverlay` | 视图与布局链路 |
| `browser/viewParts/linesDecorations/linesDecorations.ts` | `LinesDecorationsOverlay` | 视图与布局链路 |
| `browser/viewParts/margin/margin.ts` | `Margin` | 视图与布局链路 |
| `browser/viewParts/marginDecorations/marginDecorations.ts` | `MarginViewLineDecorationsOverlay` | 视图与布局链路 |
| `browser/viewParts/minimap/minimap.ts` | `Minimap` | 视图与布局链路 |
| `browser/viewParts/overlayWidgets/overlayWidgets.ts` | `ViewOverlayWidgets` | 视图与布局链路 |
| `browser/viewParts/overviewRuler/decorationsOverviewRuler.ts` | `DecorationsOverviewRuler` | 视图与布局链路 |
| `browser/viewParts/overviewRuler/overviewRuler.ts` | `OverviewRuler` | 视图与布局链路 |
| `browser/viewParts/rulers/rulers.ts` | `Rulers` | 视图与布局链路 |
| `browser/viewParts/scrollDecoration/scrollDecoration.ts` | `ScrollDecorationViewPart` | 视图与布局链路 |
| `browser/viewParts/selections/selections.ts` | `SelectionsOverlay` | 视图与布局链路 |
| `browser/viewParts/viewCursors/viewCursor.ts` | `ViewCursor` | 视图与布局链路 |
| `browser/viewParts/viewCursors/viewCursors.ts` | `ViewCursors` | 视图与布局链路 |
| `browser/viewParts/viewLines/viewLine.ts` | `ViewLine` | 视图与布局链路 |
| `browser/viewParts/viewLines/viewLineOptions.ts` | `ViewLineOptions` | 视图与布局链路 |
| `browser/viewParts/viewLines/viewLines.ts` | `ViewLines` | 视图与布局链路 |
| `browser/viewParts/viewZones/viewZones.ts` | `ViewZones` | 视图与布局链路 |
| `browser/viewParts/whitespace/whitespace.ts` | `WhitespaceOverlay` | 视图与布局链路 |
| `browser/widget/codeEditor/codeEditorContributions.ts` | `CodeEditorContributions` | 浏览器编辑器契约 |
| `browser/widget/codeEditor/codeEditorWidget.ts` | `CodeEditorWidget` | 浏览器编辑器契约 |
| `browser/widget/diffEditor/diffEditorWidget.ts` | `DiffEditorWidget` | 浏览器编辑器契约 |
| `browser/widget/multiDiffEditor/multiDiffEditorWidget.ts` | `MultiDiffEditorWidget` | 浏览器编辑器契约 |
| `common/cursor/cursor.ts` | `CursorsController` | 光标与编辑操作 |
| `common/cursor/cursorCollection.ts` | `CursorCollection` | 光标与编辑操作 |
| `common/cursor/cursorColumnSelection.ts` | `ColumnSelection` | 光标与编辑操作 |
| `common/cursor/cursorDeleteOperations.ts` | `DeleteOperations` | 光标与编辑操作 |
| `common/cursor/cursorMoveCommands.ts` | `CursorMoveCommands` | 光标与编辑操作 |
| `common/cursor/cursorMoveOperations.ts` | `MoveOperations` | 光标与编辑操作 |
| `common/cursor/cursorTypeEditOperations.ts` | `TypeWithoutInterceptorsOperation`、`AutoClosingOvertypeOperation` | 光标与编辑操作 |
| `common/cursor/cursorTypeOperations.ts` | `TypeOperations` | 光标与编辑操作 |
| `common/cursor/cursorWordOperations.ts` | `WordOperations` | 光标与编辑操作 |
| `common/cursor/oneCursor.ts` | `Cursor` | 光标与编辑操作 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | `PieceTreeTextBuffer` | 文本模型与 Piece Tree |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts` | `PieceTreeTextBufferBuilder` | 文本模型与 Piece Tree |
| `common/model/textModel.ts` | `TextModel` | 文本模型与 Piece Tree |
| `common/model.ts` | `ITextModel` | 文本模型与 Piece Tree |
| `common/services/languageFeatures.ts` | `ILanguageFeaturesService` | Editor 服务 |
| `common/services/languageFeaturesService.ts` | `LanguageFeaturesService` | Editor 服务 |
| `common/services/model.ts` | `IModelService` | Editor 服务 |
| `common/services/modelService.ts` | `ModelService` | Editor 服务 |
| `common/viewLayout/lineHeights.ts` | `CustomLineHeightData` | 无 DOM 布局 |
| `common/viewLayout/viewLayout.ts` | `ViewLayout` | 无 DOM 布局 |
| `contrib/codeAction/browser/codeActionController.ts` | `CodeActionController` | 可装配功能 |
| `contrib/codelens/browser/codelensController.ts` | `CodeLensContribution` | 可装配功能 |
| `contrib/codelens/browser/codelensWidget.ts` | `CodeLensWidget` | 可装配功能 |
| `contrib/colorPicker/browser/colorDetector.ts` | `ColorDetector` | 可装配功能 |
| `contrib/find/browser/findController.ts` | `FindController` | 可装配功能 |
| `contrib/folding/browser/folding.ts` | `FoldingController` | 可装配功能 |
| `contrib/inlayHints/browser/inlayHintsController.ts` | `InlayHintsController` | 可装配功能 |
| `contrib/inlineCompletions/browser/controller/inlineCompletionsController.ts` | `InlineCompletionsController` | 可装配功能 |
| `contrib/middleScroll/browser/middleScrollController.ts` | `MiddleScrollController` | 可装配功能 |
| `contrib/multicursor/browser/multicursor.ts` | `SelectionHighlighter` | 可装配功能 |
| `contrib/placeholderText/browser/placeholderTextContribution.ts` | `PlaceholderTextContribution` | 可装配功能 |
| `contrib/stickyScroll/browser/stickyScrollController.ts` | `StickyScrollController` | 可装配功能 |
| `contrib/suggest/browser/suggestController.ts` | `SuggestController` | 可装配功能 |
| `contrib/wordHighlighter/browser/textualHighlightProvider.ts` | `TextualMultiDocumentHighlightFeature` | 可装配功能 |
| `contrib/zoneWidget/browser/zoneWidget.ts` | `ZoneWidget` | 可装配功能 |
| `standalone/browser/standaloneEditor.ts` | `create`、`createModel`、`getModel`、`getModels`、`setModelLanguage`、`getEditors` | 独立入口 |

## 本轮已经落实

- `browser/editorBrowser.ts` 只保留上游同路径的编辑器公共契约；完整装配根、输入资源和输入控制分别由 `configuredCodeEditor.ts`、`editorInput.ts`、`editorView.ts` 负责。
- `browser/view.ts` 直接导出 `View`，所有生产调用方、测试和文档已移除 `EditorViewport` 兼容别名。
- `browser/viewParts` 已按区分大小写的上游路径落盘；本地专有 GPU、overlay、颜色弹窗、折叠装饰、预览面板和状态消息均使用明确的本地名称，不占用职责不同的上游 owner。
- `LinesLayout` 与 `LineHeightsManager` 恢复 1 基行号和 whitespace 契约；`EditorViewportLinesLayout` 只适配本地零基行号、overscan 和快照格式，行高、padding、View Zone/whitespace 排序、总高度与偏移均走 `LinesLayout`。
- `TextAreaState` 不再二次反转 RTL 选择方向，也不再保留旧调试常量别名。
- 参数提示配置在 editor 边界使用 `IEditorOptions['parameterHints']` 对象；Workbench 的布尔配置只在接入边界转换一次。
- `editor/browser/dataTransfer.ts` 负责浏览器 `DataTransfer` 到 `VSDataTransfer` 的转换；通用 MIME 留在 base，桌面文件路径解析留在 platform。
- citation 工具栏已从真实 owner `contrib/citation/common/citationCommands.ts` 导入命令，移除了不存在的聚合路径。
- `RangeUtil` 按渲染行的 `ownerDocument` 复用 DOM Range，不再错误使用全局 document；RTL 选区、光标和跨 document 编辑器都走实际所在文档的几何。

## 基础 owner 阻塞

同名差异表只统计“两边已经存在同路径声明”的项目，不包含上游存在而 Zeta 尚未建立的文件。架构测试当前明确暴露了这些基础缺口：

- `common/cursor/cursorContext.ts` 与其 `ICoordinatesConverter` / `CursorConfiguration` 依赖尚未建立；现有 `CursorControllerContext` 是另一职责，不能改名冒充。
- `browser/view/domLineBreaksComputer.ts` 尚未建立，浏览器测量后的换行计算还没有上游同 owner 的调试入口。
- `common/services/semanticTokensStylingService.ts` 尚未建立；现有 resolved-token 服务不能占用这个服务 owner。
- `browser/editorBrowser.ts` 还没有完整的 `ICodeEditor` / `IDiffEditor` 契约，因此稳定滚动、编辑器服务和多个 contrib 仍不能按上游调用链收口。
- 上游 `IClipboardCopyEvent` / `createClipboardCopyEvent` 依赖 ViewModel 生成 `dataToCopy` 并负责写入编辑器元数据；本地现有 DOM 包装已改为 `IEditorClipboardCopyEvent` / `createEditorClipboardCopyEvent`，不再冒充该能力。

## 验证状态

- 本轮新增定向回归 83 项通过：31 项覆盖 `LinesLayout`、View Zone、viewport resize、稳定滚动与剪贴板，48 项覆盖完整 View（含 RTL DOM 几何），4 项覆盖 citation 渲染和工具栏命令。
- `tsconfig.renderer.json` 与 `tsconfig.test.json` 的 editor 范围已无类型错误；全仓仍被生成协议与调用端漂移阻塞。
- Editor 架构测试 17 项通过、4 项失败；失败只对应上面列出的 `cursorContext.ts`、`domLineBreaksComputer.ts`、`semanticTokensStylingService.ts` 基础缺口，没有放宽结构检查。
- 下一批先补 `ITextModel` / `IModelService` / `ICodeEditor` 基础契约，再回到稳定滚动、视图部件和 contrib 调用链，避免在下游继续堆局部适配。
