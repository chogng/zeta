# Editor API 对齐状态

> 本表记录 2026-08-30 对 `zeta-ts/src/zeta/editor` 全部生产 TypeScript 文件的扫描结果。分层依据为 VS Code 的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 和仓库内 `vscode-api-alignment` skill。

## 结论

- 已扫描 486 个本地生产文件，并与 661 个 VS Code Editor 生产文件比较。
- 明确的 import owner 路径错误：0；全局唯一 owner 冲突：0；同 basename 放错路径候选：0。
- 仍有 118 个同名声明的结构差异，分布在 106 个文件中。它们都是同一职责下尚未补齐的实现，不能只改参数名或返回类型伪装成已完成。
- import 集合不同不单独判错：缺少上游能力会自然缺少对应 import，本地真实扩展也会增加 import；只有同一符号从错误 owner 导入才属于路径错误。

## 处理规则

1. 上游同名、同职责且本地能力足够时，直接恢复上游名称、参数、返回值、owner 与调用链。
2. 本地职责与上游同名声明不一致时，移出上游 owner 或改成清楚的本地名称；上游能力保持“未实现”，不留同名壳子。
3. 下表中的差异保留在上游 owner，是因为它们确实属于同一职责，但底层模型、视图上下文、服务或渲染协议尚未齐全；后续应从基础依赖向调用方逐层实现。

## 尚未补齐的同名契约

| 文件 | 声明 | 分类 |
| --- | --- | --- |
| `browser/controller/dragScrolling.ts` | `DragScrolling` | 浏览器编辑器契约 |
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardCopyEvent`、`IClipboardPasteEvent`、`createClipboardCopyEvent` | 输入与无障碍链路 |
| `browser/controller/editContext/editContext.ts` | `AbstractEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/native/debugEditContext.ts` | `DebugEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/native/nativeEditContext.ts` | `NativeEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/native/nativeEditContextUtils.ts` | `FocusTracker` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderContentRich.ts` | `RichScreenReaderContent` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderContentSimple.ts` | `SimpleScreenReaderContent` | 输入与无障碍链路 |
| `browser/controller/editContext/native/screenReaderSupport.ts` | `ScreenReaderSupport` | 输入与无障碍链路 |
| `browser/controller/editContext/textArea/textAreaEditContext.ts` | `TextAreaEditContext` | 输入与无障碍链路 |
| `browser/controller/editContext/textArea/textAreaEditContextInput.ts` | `TextAreaInput` | 输入与无障碍链路 |
| `browser/controller/mouseHandler.ts` | `MouseHandler` | 浏览器编辑器契约 |
| `browser/gpu/atlas/textureAtlas.ts` | `TextureAtlas` | GPU 渲染链路 |
| `browser/gpu/atlas/textureAtlasPage.ts` | `TextureAtlasPage` | GPU 渲染链路 |
| `browser/gpu/atlas/textureAtlasShelfAllocator.ts` | `TextureAtlasShelfAllocator` | GPU 渲染链路 |
| `browser/gpu/atlas/textureAtlasSlabAllocator.ts` | `TextureAtlasSlabAllocator` | GPU 渲染链路 |
| `browser/gpu/raster/glyphRasterizer.ts` | `GlyphRasterizer` | GPU 渲染链路 |
| `browser/gpu/rectangleRenderer.ts` | `RectangleRenderer` | GPU 渲染链路 |
| `browser/gpu/renderStrategy/baseRenderStrategy.ts` | `BaseRenderStrategy` | GPU 渲染链路 |
| `browser/gpu/renderStrategy/fullFileRenderStrategy.ts` | `FullFileRenderStrategy` | GPU 渲染链路 |
| `browser/gpu/renderStrategy/viewportRenderStrategy.ts` | `ViewportRenderStrategy` | GPU 渲染链路 |
| `browser/gpu/viewGpuContext.ts` | `ViewGpuContext` | GPU 渲染链路 |
| `browser/observableCodeEditor.ts` | `observableCodeEditor`、`ObservableCodeEditor` | 浏览器编辑器契约 |
| `browser/services/abstractCodeEditorService.ts` | `AbstractCodeEditorService` | 浏览器编辑器契约 |
| `browser/services/codeEditorService.ts` | `ICodeEditorOpenHandler`、`ICodeEditorService` | 浏览器编辑器契约 |
| `browser/stableEditorScroll.ts` | `StableEditorScrollState`、`StableEditorBottomScrollState` | 浏览器编辑器契约 |
| `browser/view/dynamicViewOverlay.ts` | `DynamicViewOverlay` | 视图与布局链路 |
| `browser/view/viewController.ts` | `ViewController` | 视图与布局链路 |
| `browser/view/viewOverlays.ts` | `ViewOverlays` | 视图与布局链路 |
| `browser/view/viewUserInputEvents.ts` | `ViewUserInputEvents` | 视图与布局链路 |
| `browser/view.ts` | `View` | 视图与布局链路 |
| `browser/viewparts/blockDecorations/blockDecorations.ts` | `BlockDecorations` | 视图与布局链路 |
| `browser/viewparts/contentWidgets/contentWidgets.ts` | `ViewContentWidgets` | 视图与布局链路 |
| `browser/viewparts/currentLineHighlight/currentLineHighlight.ts` | `CurrentLineHighlightOverlay` | 视图与布局链路 |
| `browser/viewparts/decorations/decorations.ts` | `DecorationsOverlay` | 视图与布局链路 |
| `browser/viewparts/editorScrollbar/editorScrollbar.ts` | `EditorScrollbar` | 视图与布局链路 |
| `browser/viewparts/glyphMargin/glyphMargin.ts` | `GlyphMarginWidgets` | 视图与布局链路 |
| `browser/viewparts/gpuMark/gpuMark.ts` | `GpuMarkOverlay` | 视图与布局链路 |
| `browser/viewparts/indentGuides/indentGuides.ts` | `IndentGuidesOverlay` | 视图与布局链路 |
| `browser/viewparts/lineNumbers/lineNumbers.ts` | `LineNumbersOverlay` | 视图与布局链路 |
| `browser/viewparts/linesDecorations/linesDecorations.ts` | `LinesDecorationsOverlay` | 视图与布局链路 |
| `browser/viewparts/margin/margin.ts` | `Margin` | 视图与布局链路 |
| `browser/viewparts/marginDecorations/marginDecorations.ts` | `MarginViewLineDecorationsOverlay` | 视图与布局链路 |
| `browser/viewparts/minimap/minimap.ts` | `Minimap` | 视图与布局链路 |
| `browser/viewparts/overlayWidgets/overlayWidgets.ts` | `ViewOverlayWidgets` | 视图与布局链路 |
| `browser/viewparts/overviewRuler/decorationsOverviewRuler.ts` | `DecorationsOverviewRuler` | 视图与布局链路 |
| `browser/viewparts/overviewRuler/overviewRuler.ts` | `OverviewRuler` | 视图与布局链路 |
| `browser/viewparts/rulers/rulers.ts` | `Rulers` | 视图与布局链路 |
| `browser/viewparts/rulersGpu/rulersGpu.ts` | `RulersGpu` | 视图与布局链路 |
| `browser/viewparts/scrollDecoration/scrollDecoration.ts` | `ScrollDecorationViewPart` | 视图与布局链路 |
| `browser/viewparts/selections/selections.ts` | `SelectionsOverlay` | 视图与布局链路 |
| `browser/viewparts/viewCursors/viewCursor.ts` | `ViewCursor` | 视图与布局链路 |
| `browser/viewparts/viewCursors/viewCursors.ts` | `ViewCursors` | 视图与布局链路 |
| `browser/viewparts/viewLines/viewLine.ts` | `ViewLine` | 视图与布局链路 |
| `browser/viewparts/viewLines/viewLineOptions.ts` | `ViewLineOptions` | 视图与布局链路 |
| `browser/viewparts/viewLines/viewLines.ts` | `ViewLines` | 视图与布局链路 |
| `browser/viewparts/viewLinesGpu/viewLinesGpu.ts` | `ViewLinesGpu` | 视图与布局链路 |
| `browser/viewparts/viewZones/viewZones.ts` | `ViewZones` | 视图与布局链路 |
| `browser/viewparts/whitespace/whitespace.ts` | `WhitespaceOverlay` | 视图与布局链路 |
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
| `common/services/languageService.ts` | `LanguageService` | Editor 服务 |
| `common/services/model.ts` | `IModelService` | Editor 服务 |
| `common/services/modelService.ts` | `ModelService` | Editor 服务 |
| `common/viewLayout/lineHeights.ts` | `CustomLineHeightData`、`LineHeightsManager` | 无 DOM 布局 |
| `common/viewLayout/linesLayout.ts` | `LinesLayout` | 无 DOM 布局 |
| `common/viewLayout/viewLayout.ts` | `ViewLayout` | 无 DOM 布局 |
| `contrib/codeAction/browser/codeActionController.ts` | `CodeActionController` | 可装配功能 |
| `contrib/codelens/browser/codelensController.ts` | `CodeLensContribution` | 可装配功能 |
| `contrib/codelens/browser/codelensWidget.ts` | `CodeLensWidget` | 可装配功能 |
| `contrib/colorPicker/browser/colorDetector.ts` | `ColorDetector` | 可装配功能 |
| `contrib/colorPicker/browser/colorPickerModel.ts` | `ColorPickerModel` | 可装配功能 |
| `contrib/colorPicker/browser/colorPickerWidget.ts` | `ColorPickerWidget` | 可装配功能 |
| `contrib/find/browser/findController.ts` | `FindController` | 可装配功能 |
| `contrib/folding/browser/folding.ts` | `FoldingController` | 可装配功能 |
| `contrib/folding/browser/foldingDecorations.ts` | `FoldingDecorationProvider` | 可装配功能 |
| `contrib/inlayHints/browser/inlayHintsController.ts` | `InlayHintsController` | 可装配功能 |
| `contrib/inlineCompletions/browser/controller/inlineCompletionsController.ts` | `InlineCompletionsController` | 可装配功能 |
| `contrib/message/browser/messageController.ts` | `MessageController` | 可装配功能 |
| `contrib/middleScroll/browser/middleScrollController.ts` | `MiddleScrollController` | 可装配功能 |
| `contrib/multicursor/browser/multicursor.ts` | `SelectionHighlighter` | 可装配功能 |
| `contrib/peekView/browser/peekView.ts` | `PeekViewWidget` | 可装配功能 |
| `contrib/placeholderText/browser/placeholderTextContribution.ts` | `PlaceholderTextContribution` | 可装配功能 |
| `contrib/stickyScroll/browser/stickyScrollController.ts` | `StickyScrollController` | 可装配功能 |
| `contrib/suggest/browser/suggestController.ts` | `SuggestController` | 可装配功能 |
| `contrib/wordHighlighter/browser/textualHighlightProvider.ts` | `TextualMultiDocumentHighlightFeature` | 可装配功能 |
| `contrib/zoneWidget/browser/zoneWidget.ts` | `ZoneWidget` | 可装配功能 |
| `standalone/browser/standaloneEditor.ts` | `create`、`createModel`、`getModel`、`getModels`、`setModelLanguage`、`getEditors` | 独立入口 |

## 已清除的误导性同名职责

- DOM 命中分类：`SemanticMouseTargetFactory` / `semanticMouseTarget.ts`。
- 浏览器 pointer 事件路由：`PointerEventRouter` / `pointerEventRouter.ts`。
- Zeta viewport 渲染快照：`EditorViewportData` / `editorViewportData.ts`。
- Zeta Piece 数据结构：`BufferPiece`。
- Zeta provider CodeLens 聚合：`LanguageCodeLensModel`、`LanguageCodeLensItem`。
- 命名颜色主题：`NamedEditorThemeService` / `namedEditorTheme.ts`。
- 独立编辑器实例：`StandaloneEditorInstance`。
- 带 language IDs 的本地 provider 注册入口：`registerLanguage*Provider`；已具备上游协议的 `registerDocumentHighlightProvider` 保留上游名称。
