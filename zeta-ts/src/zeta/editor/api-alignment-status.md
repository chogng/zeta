# Editor API 对齐状态

> 本表记录 2026-08-30 对 `zeta-ts/src/zeta/editor` 生产 TypeScript 文件的扫描结果。分层依据为 VS Code 的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 和仓库内 `vscode-api-alignment` skill。

## 当前结论

- 已扫描 491 个本地生产文件，并与 661 个 VS Code Editor 生产文件比较。
- 初始确认 118 组同名声明结构差异；已处理 106 组，剩余 12 组。此前通过改成本地名称隐藏的 cursor 与 observable editor 差异已经重新纳入待处理项。
- 当前没有已知的 import owner 路径错误、全局唯一 owner 冲突或同 basename 放错路径候选。
- import 集合不同不单独判错：缺少上游能力会自然缺少对应 import，本地真实扩展也会增加 import。只有同一符号从错误 owner 导入才属于路径错误。
- 输入、参数和返回类型一致不代表行为已经一致。剩余项仍需继续核对状态 owner、事件顺序、失效条件、调度阶段、坐标转换、可见副作用、失败语义和释放时机。

## 处理规则

1. 同名、同职责且本地能力完整时，恢复上游名称、参数、返回值、owner 和调用链。
2. 本地职责与上游同名声明不同，且它确实是 Zeta 专有能力时，移出上游 owner 或改成明确的本地名称；不保留别名、包装入口或重复导出。
3. 缺少基础模型、视图上下文或服务契约时，从基础 owner 向调用方逐层实现，不在下游伪造同名接口。
4. `editor` 可以依赖 `base` 和 `platform`；`common` 不能依赖 DOM，`browser` 才能使用 DOM，依赖方向不能反转。

## 已处理的同名契约

| 文件 | 声明 | 结果 |
| --- | --- | --- |
| `browser/controller/dragScrolling.ts` | `DragScrolling` | 本地职责已改名或移出上游 owner |
| `browser/controller/mouseHandler.ts` | `MouseHandler` | 本地 pointer/selection owner 改为 `EditorPointerSelectionHandler` |
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardCopyEvent` | 本地职责已改名或移出上游 owner |
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardPasteEvent` | 已按上游契约对齐 |
| `browser/controller/editContext/clipboardUtils.ts` | `createClipboardCopyEvent` | 本地职责已改名或移出上游 owner |
| `browser/controller/editContext/native/debugEditContext.ts` | `DebugEditContext` | 本地职责已改名或移出上游 owner |
| `browser/controller/editContext/native/nativeEditContextUtils.ts` | `FocusTracker` | 本地 DOM 焦点助手改为明确的 `EditContextFocusTracker` |
| `browser/controller/editContext/editContext.ts` | `AbstractEditContext` | 本地输入路由与 composition owner 改为 `EditorInputContext` |
| `browser/controller/editContext/native/nativeEditContext.ts` | `NativeEditContext` | 本地浏览器 EditContext 实现改为 `BrowserEditContext` |
| `browser/controller/editContext/native/screenReaderContentRich.ts` | `RichScreenReaderContent` | 本地无障碍镜像实现改为 `EditorRichScreenReaderContent` |
| `browser/controller/editContext/native/screenReaderContentSimple.ts` | `SimpleScreenReaderContent` | 本地无障碍镜像实现改为 `EditorSimpleScreenReaderContent` |
| `browser/controller/editContext/native/screenReaderSupport.ts` | `ScreenReaderSupport` | 本地输入层无障碍协调器改为 `EditorScreenReaderSupport` |
| `browser/controller/editContext/textArea/textAreaEditContext.ts` | `TextAreaEditContext` | 本地 textarea 输入上下文改为 `EditorTextAreaInputContext` |
| `browser/controller/editContext/textArea/textAreaEditContextInput.ts` | `TextAreaInput` | 本地 DOM 事件适配器改为 `EditorTextAreaInput` |
| `browser/gpu/atlas/textureAtlas.ts` | `TextureAtlas` | 本地职责已改名或移出上游 owner |
| `browser/gpu/atlas/textureAtlasPage.ts` | `TextureAtlasPage` | 本地职责已改名或移出上游 owner |
| `browser/gpu/atlas/textureAtlasShelfAllocator.ts` | `TextureAtlasShelfAllocator` | 本地职责已改名或移出上游 owner |
| `browser/gpu/atlas/textureAtlasSlabAllocator.ts` | `TextureAtlasSlabAllocator` | 本地职责已改名或移出上游 owner |
| `browser/gpu/raster/glyphRasterizer.ts` | `GlyphRasterizer` | 本地职责已改名或移出上游 owner |
| `browser/gpu/rectangleRenderer.ts` | `RectangleRenderer` | 本地职责已改名或移出上游 owner |
| `browser/gpu/raster/baseRenderStrategy.ts` | `BaseRenderStrategy` | 本地职责已改名或移出上游 owner |
| `browser/gpu/raster/fullFileRenderStrategy.ts` | `FullFileRenderStrategy` | 本地职责已改名或移出上游 owner |
| `browser/gpu/raster/viewportRenderStrategy.ts` | `ViewportRenderStrategy` | 本地职责已改名或移出上游 owner |
| `browser/gpu/viewGpuContext.ts` | `ViewGpuContext` | 本地职责已改名或移出上游 owner |
| `browser/observableCodeEditor.ts` | `observableCodeEditor` | 已恢复上游公开名、单参数入口和单例 facade 身份 |
| `browser/viewParts/overlayWidgets/overlayWidgets.ts` | `ViewOverlayWidgets` | 已恢复上游公开名、成员边界、DOM owner、配置更新、布局缓存和 widget 生命周期 |
| `browser/view/dynamicViewOverlay.ts` | `DynamicViewOverlay` | 本地职责已改名或移出上游 owner |
| `browser/view/viewOverlays.ts` | `ViewOverlays` | 本地职责已改名或移出上游 owner |
| `browser/view/viewController.ts` | `ViewController` | 本地输入命令协调器改为 `EditorViewInputController` |
| `browser/view/viewUserInputEvents.ts` | `ViewUserInputEvents` | 本地输入事件 facade 改为 `EditorViewUserInputEvents` |
| `browser/viewParts/gpuMark/gpuMark.ts` | `GpuMarkOverlay` | 本地职责已改名或移出上游 owner |
| `browser/viewParts/rulersGpu/rulersGpu.ts` | `RulersGpu` | 本地职责已改名或移出上游 owner |
| `browser/stableEditorScroll.ts` | `StableEditorScrollState`、`StableEditorBottomScrollState` | 本地 `View` 边界实现改为明确的 `ViewStableEditorScrollState` / `ViewStableEditorBottomScrollState` |
| `browser/viewParts/blockDecorations/blockDecorations.ts` | `BlockDecorations` | 本地 scheduler 实现改为 `EditorBlockDecorations` |
| `browser/viewParts/contentWidgets/contentWidgets.ts` | `ViewContentWidgets` | 本地 scheduler 实现改为 `EditorContentWidgets` |
| `browser/viewParts/currentLineHighlight/currentLineHighlight.ts` | `CurrentLineHighlightOverlay` | 本地 scheduler 实现改为 `EditorCurrentLineHighlightOverlay` |
| `browser/viewParts/decorations/decorations.ts` | `DecorationsOverlay` | 本地 scheduler 实现改为 `EditorDecorationsOverlay` |
| `browser/viewParts/editorScrollbar/editorScrollbar.ts` | `EditorScrollbar` | 本地 scheduler 实现改为 `EditorViewportScrollbar` |
| `browser/viewParts/glyphMargin/glyphMargin.ts` | `GlyphMarginWidgets` | 本地 scheduler 实现改为 `EditorGlyphMarginWidgets` |
| `browser/viewParts/indentGuides/indentGuides.ts` | `IndentGuidesOverlay` | 本地 scheduler 实现改为 `EditorIndentGuidesOverlay` |
| `browser/viewParts/lineNumbers/lineNumbers.ts` | `LineNumbersOverlay` | 本地 scheduler 实现改为 `EditorLineNumbersOverlay` |
| `browser/viewParts/linesDecorations/linesDecorations.ts` | `LinesDecorationsOverlay` | 本地 scheduler 实现改为 `EditorLinesDecorationsOverlay` |
| `browser/viewParts/margin/margin.ts` | `Margin` | 本地 scheduler 实现改为 `EditorMargin` |
| `browser/viewParts/marginDecorations/marginDecorations.ts` | `MarginViewLineDecorationsOverlay` | 本地 scheduler 实现改为 `EditorMarginLineDecorationsOverlay` |
| `browser/viewParts/minimap/minimap.ts` | `Minimap` | 本地 scheduler 实现改为 `EditorMinimap` |
| `browser/viewParts/overviewRuler/decorationsOverviewRuler.ts` | `DecorationsOverviewRuler` | 本地 scheduler 实现改为 `EditorDecorationsOverviewRuler` |
| `browser/viewParts/overviewRuler/overviewRuler.ts` | `OverviewRuler` | 本地 scheduler 实现改为 `EditorOverviewRuler` |
| `browser/viewParts/rulers/rulers.ts` | `Rulers` | 本地 scheduler 实现改为 `EditorRulers` |
| `browser/viewParts/scrollDecoration/scrollDecoration.ts` | `ScrollDecorationViewPart` | 本地 scheduler 实现改为 `EditorScrollDecorationViewPart` |
| `browser/viewParts/selections/selections.ts` | `SelectionsOverlay` | 本地 scheduler 实现改为 `EditorSelectionsOverlay` |
| `browser/viewParts/viewCursors/viewCursor.ts` | `ViewCursor` | 本地 scheduler 实现改为 `EditorViewCursor` |
| `browser/viewParts/viewCursors/viewCursors.ts` | `ViewCursors` | 本地 scheduler 实现改为 `EditorViewCursors` |
| `browser/viewParts/viewLines/viewLine.ts` | `ViewLine` | 本地 scheduler 实现改为 `EditorViewLine` |
| `browser/viewParts/viewLines/viewLineOptions.ts` | `ViewLineOptions` | 本地 scheduler 实现改为 `EditorViewLineOptions` |
| `browser/viewParts/viewLines/viewLines.ts` | `ViewLines` | 本地 scheduler 实现改为 `EditorViewLines` |
| `browser/viewParts/viewZones/viewZones.ts` | `ViewZones` | 本地 scheduler 实现改为 `EditorViewZones` |
| `browser/viewParts/whitespace/whitespace.ts` | `WhitespaceOverlay` | 本地 scheduler 实现改为 `EditorWhitespaceOverlay` |
| `browser/widget/codeEditor/codeEditorContributions.ts` | `CodeEditorContributions` | 本地多 context contribution owner 改为 `WidgetContributionCollection` |
| `browser/widget/diffEditor/diffEditorWidget.ts` | `DiffEditorWidget` | 本地只读虚拟化审阅面板改为 `EditorDiffWidget` |
| `browser/widget/multiDiffEditor/multiDiffEditorWidget.ts` | `MultiDiffEditorWidget` | 本地多文件审阅面板改为 `EditorMultiDiffWidget` |
| `browser/services/abstractCodeEditorService.ts` | `AbstractCodeEditorService` | 本地 Widget 注册表实现改为 `AbstractWidgetCodeEditorRegistry` |
| `browser/services/codeEditorService.ts` | `ICodeEditorOpenHandler` | 本地 URI open handler 改为 `IWidgetCodeEditorOpenHandler` |
| `browser/services/codeEditorService.ts` | `ICodeEditorService` | 本地 Widget 注册表契约改为 `IWidgetCodeEditorRegistry` |
| `browser/viewParts/viewLinesGpu/viewLinesGpu.ts` | `ViewLinesGpu` | 本地职责已改名或移出上游 owner |
| `common/services/languageService.ts` | `LanguageService` | 已按上游契约对齐 |
| `common/services/languageFeatures.ts` | `ILanguageFeaturesService` | 本地 provider 集合契约改为 `IEditorLanguageFeaturesService` |
| `common/services/languageFeaturesService.ts` | `LanguageFeaturesService` | 本地 provider registry 改为 `EditorLanguageFeaturesService` |
| `common/viewLayout/lineHeights.ts` | `LineHeightsManager` | 已按上游契约对齐 |
| `common/viewLayout/linesLayout.ts` | `LinesLayout` | 已按上游契约对齐 |
| `common/viewLayout/lineHeights.ts` | `CustomLineHeightData` | 本地 scheduler 数据改为 `EditorCustomLineHeightData` |
| `common/viewLayout/viewLayout.ts` | `ViewLayout` | 本地 viewport layout owner 改为 `EditorViewportLayoutManager` |
| `common/cursor/cursorTypeEditOperations.ts` | `TypeWithoutInterceptorsOperation` | 已恢复上游公开名与成员边界；selection offset 归并留在文件私有 helper，不再伪装成 class API |
| `common/cursor/cursorTypeEditOperations.ts` | `AutoClosingOvertypeOperation` | 已恢复上游公开名与 `_runAutoClosingOvertype` 内部阶段，现有多光标 overtype 行为保持不变 |
| `common/cursor/cursorColumnSelection.ts` | `ColumnSelection` | 已恢复上游 5 个公开方法；鼠标列选通过 `CursorConfiguration`、视觉行模型和 `ICoordinatesConverter` 往返模型坐标，方向与短行行为由生产调用链测试覆盖 |
| `common/cursor/cursorMoveOperations.ts` | `MoveOperations` | 成员边界与上游比较为 0；字符、折行、visible column、sticky tab stop、上下移动、空行与 buffer 边界行为使用 `CursorConfiguration` 和 `ICursorSimpleModel`。本地 `SelectionSet` 键盘导航已迁到 `cursorNavigation.ts` |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | `PieceTreeTextBuffer` | 已恢复 `common/model.ts` 的 `ITextBuffer` owner、1-based Position/Range 查询、原子编辑与逆编辑、逐行搜索、内容事件、BOM/EOL 和释放契约；红黑树内部结构保持本地实现 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts` | `PieceTreeTextBufferBuilder` | 已按上游两阶段 builder/factory 契约对齐 |
| `common/services/model.ts` | `IModelService` | 已恢复 `ITextModel`、buffer factory、creation options 与 edit source 契约 |
| `common/services/modelService.ts` | `ModelService` | 模型创建配置改由 `platform/configuration` 注入；语言/资源配置失效会更新现有模型，EOL 由 `ITextResourcePropertiesService` 决定，关闭文件的 undo/redo 以 SHA-1 校验内容并受内存上限约束 |
| `contrib/colorPicker/browser/colorPickerModel.ts` | `ColorPickerModel` | 已按上游契约对齐 |
| `contrib/colorPicker/browser/colorPickerWidget.ts` | `ColorPickerWidget` | 本地职责已改名或移出上游 owner |
| `contrib/folding/browser/foldingDecorations.ts` | `FoldingDecorationProvider` | 本地职责已改名或移出上游 owner |
| `contrib/message/browser/messageController.ts` | `MessageController` | 本地职责已改名或移出上游 owner |
| `contrib/middleScroll/browser/middleScrollController.ts` | `MiddleScrollController` | 本地 pointer capture 拖拽平移改为明确的 `PointerMiddleScrollController` |
| `contrib/peekView/browser/peekView.ts` | `PeekViewWidget` | 本地职责已改名或移出上游 owner |
| `contrib/placeholderText/browser/placeholderTextContribution.ts` | `PlaceholderTextContribution` | 具体 Widget/DOM 生命周期实现改为 `WidgetPlaceholderTextContribution` |
| `contrib/codeAction/browser/codeActionController.ts` | `CodeActionController` | 本地 contribution 实现改为 `EditorCodeActionController` |
| `contrib/codelens/browser/codelensController.ts` | `CodeLensContribution` | 本地 contribution 实现改为 `EditorCodeLensContribution` |
| `contrib/codelens/browser/codelensWidget.ts` | `CodeLensWidget` | 本地 contribution Widget 改为 `EditorCodeLensWidget` |
| `contrib/colorPicker/browser/colorDetector.ts` | `ColorDetector` | 本地 contribution 实现改为 `EditorColorDetector` |
| `contrib/find/browser/findController.ts` | `FindController` | 本地 contribution 实现改为 `EditorFindController` |
| `contrib/folding/browser/folding.ts` | `FoldingController` | 本地 contribution 实现改为 `EditorFoldingController` |
| `contrib/inlayHints/browser/inlayHintsController.ts` | `InlayHintsController` | 本地 contribution 实现改为 `EditorInlayHintsController` |
| `contrib/inlineCompletions/browser/controller/inlineCompletionsController.ts` | `InlineCompletionsController` | 本地 contribution 实现改为 `EditorInlineCompletionsController` |
| `contrib/multicursor/browser/multicursor.ts` | `SelectionHighlighter` | 本地 occurrence 高亮实现改为 `EditorSelectionHighlighter` |
| `contrib/stickyScroll/browser/stickyScrollController.ts` | `StickyScrollController` | 本地 contribution 实现改为 `EditorStickyScrollController` |
| `contrib/suggest/browser/suggestController.ts` | `SuggestController` | 本地 contribution 实现改为 `EditorSuggestController` |
| `contrib/zoneWidget/browser/zoneWidget.ts` | `ZoneWidget` | 本地 View/DOM 基类改为 `EditorZoneWidget` |
| `contrib/wordHighlighter/browser/textualHighlightProvider.ts` | `TextualMultiDocumentHighlightFeature` | 单模型 target 生命周期改为明确的 `TextualHighlightTargetRegistration` |
| `standalone/browser/standaloneEditor.ts` | `createModel`、`getModel`、`getModels`、`setModelLanguage` | 公共模型边界已改用 `ITextModel` |
| `standalone/browser/standaloneEditor.ts` | `getEditors` | 改由 `IWidgetCodeEditorRegistry` 统一持有并返回实际编辑器对象 |
| `standalone/browser/standaloneEditor.ts` | `create` | 返回 `standaloneCodeEditor.ts` 的 `StandaloneEditor`；`create()`、`onDidCreateEditor()` 与 `getEditors()` 现在共享同一对象身份和释放时机，不再把内部 widget 当成另一个编辑器暴露 |

## 尚未补齐的同名契约

| 文件 | 声明 | 分类 |
| --- | --- | --- |
| `browser/observableCodeEditor.ts` | `ObservableCodeEditor` | 已恢复上游公开名；当前仍直接依赖 `CodeEditorWidget`，需在 `ICodeEditor` owner 完整后继续收口成员与生命周期 |
| `browser/view.ts` | `View` | 视图与布局链路 |
| `browser/widget/codeEditor/codeEditorWidget.ts` | `CodeEditorWidget` | 浏览器编辑器契约 |
| `common/cursor/cursor.ts` | `CursorsController` | 已恢复上游公开名；ViewModel 与 cursor context 调用链尚未补齐 |
| `common/cursor/cursorCollection.ts` | `CursorCollection` | 成员边界、primary-first 状态、marker 生命周期和 overlap normalize 已与上游一致；仍需随统一 `ViewModel` 接入生产 `CursorsController` |
| `common/cursor/cursorDeleteOperations.ts` | `DeleteOperations` | 成员边界比较为 0，已恢复 `CursorConfiguration`、`Selection[]`、`ICommand` 和自动闭合范围语义；本地 `SelectionSet` 事务位于 `selectionSetDeleteOperations.ts`。仍需随统一 `CursorsController` 接入生产编辑链 |
| `common/cursor/cursorMoveCommands.ts` | `CursorMoveCommands` | 已恢复上游公开名；命令参数与 ViewModel 调用链仍需核对 |
| `common/cursor/cursorTypeOperations.ts` | `TypeOperations` | 已恢复上游公开名；输入策略仍需逐项核对 |
| `common/cursor/cursorWordOperations.ts` | `WordOperations` | 成员边界比较为 0，word start/end、word part、删除范围和 `SingleCursorState` 选词行为已使用上游契约并有直接测试；本地正则词选区与 `SelectionSet` 事务分别位于 `wordSelection.ts`、`selectionSetWordOperations.ts`。仍需随统一 `CursorsController` 接入生产编辑链 |
| `common/cursor/oneCursor.ts` | `Cursor` | 成员边界、`modelState` / `viewState`、tracked range 与折行坐标转换已与上游一致；仍需随统一 `ViewModel` 进入生产生命周期 |
| `common/model/textModel.ts` | `TextModel` | 文本模型与 Piece Tree |
| `common/model.ts` | `ITextModel` | 文本模型与 Piece Tree |

## 本轮已经落实

- `browser/editorBrowser.ts` 只保留上游同路径的编辑器公共契约；完整装配根、输入资源和输入控制分别由 `configuredCodeEditor.ts`、`editorInput.ts`、`editorView.ts` 负责。
- `browser/view.ts` 直接导出 `View`，所有生产调用方、测试和文档已移除 `EditorViewport` 兼容别名。
- `browser/viewParts` 已按区分大小写的上游路径落盘；`ViewOverlayWidgets` 已恢复上游公开名与成员边界，DOM owner、配置更新、布局缓存和 widget 生命周期均由该 owner 负责。本地专有 GPU、颜色弹窗、折叠装饰、预览面板和状态消息仍使用明确的本地名称。
- cursor owner 组已移除 `EditorSelection*` / `ModelColumnSelection` 等本地公开名。`CursorConfiguration`、`CursorContext`、`ICursorSimpleModel` 和投影坐标转换已经建立；`Cursor` 与 `CursorCollection` 的成员比较为 0，直接测试覆盖 marker 恢复、primary-first、重叠合并和折行 model/view state。生产 `CursorsController` 尚未改由统一 `ViewModel` 持有，因此这两项仍留在待处理表。
- `MoveOperations` 已恢复上游完整成员和状态输入；原先混在该 class 里的 `SelectionSet` 导航职责迁到 `cursorNavigation.ts`。base `strings` 提供同路径的 grapheme `nextCharLength`、`prevCharLength` 与左删 offset，因此 common cursor 不再反向持有另一套字符边界。
- `DeleteOperations` 与 `WordOperations` 的成员比较均为 0；上游命令使用 `editorCommon.ts` 的 `ICommand` 和 `commands/replaceCommand.ts`，本地 `SelectionSet` 到事务命令的转换使用显式的 `selectionSetDeleteOperations.ts`、`selectionSetWordOperations.ts`。浏览器正则词选区位于 `wordSelection.ts`，不再伪装成上游 `WordOperations` 成员。
- `LinesLayout` 与 `LineHeightsManager` 恢复 1 基行号和 whitespace 契约；`EditorViewportLinesLayout` 只适配本地零基行号、overscan 和快照格式，行高、padding、View Zone/whitespace 排序、总高度与偏移均走 `LinesLayout`。
- `TextAreaState` 不再二次反转 RTL 选择方向，也不再保留旧调试常量别名。
- 参数提示配置在 editor 边界使用 `IEditorOptions['parameterHints']` 对象；Workbench 的布尔配置只在接入边界转换一次。
- `editor/browser/dataTransfer.ts` 负责浏览器 `DataTransfer` 到 `VSDataTransfer` 的转换；通用 MIME 留在 base，桌面文件路径解析留在 platform。
- citation 工具栏已从真实 owner `contrib/citation/common/citationCommands.ts` 导入命令，移除了不存在的聚合路径。
- `RangeUtil` 按渲染行的 `ownerDocument` 复用 DOM Range，不再错误使用全局 document；RTL 选区、光标和跨 document 编辑器都走实际所在文档的几何。
- `ITextModel` 已恢复读取、配置、可变 EOL、`isDisposed()`、attached-view 计数/事件和内部 tracked-range ID 契约；View 会把可见逻辑行写入实际 attached handle，并在释放时 detach。尚未实现的 model-owned decoration、tokenization、完整 ViewModel 注册与完整编辑栈契约仍保留在待处理项中，不用空声明冒充完成。
- `ModelService` 现在由 `platform/configuration` 和 `ITextResourcePropertiesService` 驱动；相关配置变化会更新已打开模型，关闭文件 history 通过 SHA-1、URI 策略和内存预算决定是否恢复。模型配置键由 editor common owner 注册，Workbench 只消费，不再反向拥有。
- `standaloneEditor.create` 返回 `standaloneCodeEditor.ts` 的 `StandaloneEditor`；创建事件、`getEditors()` 和调用者持有的是同一对象，内部 `CodeEditorWidget` 不再冒充第二个独立编辑器。
- `standaloneLanguages.ts` 的公共 provider 注册已恢复上游名称和 `(LanguageSelector, provider)` 输入边界；selector 只在入口转换成内部 registry 所需的 `languageIds` owner 元数据，旧的 `registerLanguage*Provider` 和 provider 内嵌 selector 写法已经移除。Zeta 的 worker provider batch、syntax provider 与跨文档 highlight 仍以明确的扩展入口存在。
- `TextModel.createSnapshot` 已恢复上游的顺序读取协议；本地语言请求需要的版本化随机读取快照迁到独立 `createVersionedSnapshot`，Editor 与 Workbench 调用方已按职责迁移。
- `common/coordinatesConverter.ts` 已建立上游同 owner 的 `ICoordinatesConverter` 和真实的 `IdentityCoordinatesConverter`；`ViewModelLines.createCoordinatesConverter()` 使用同一份折行与隐藏行映射完成 model/view 往返，逻辑行换行输入由独立 `ILineBreaksComputerContext` 提供，不与视觉行 `getLineContent` 混用。
- `browser/services/editorWorkerService.ts` 现在持有生产使用的 `VersionedEditorWorkerClient`，浏览器 Worker factory、Configured Editor 和相关 contribution 都从该 owner 导入；旧 `versionedEditorWorkerClient.ts` 已删除。公共 worker 启动入口也已迁到 `editor.worker.start.ts` 的 `start()`，三个 worker main 使用同一路径。
- `browser/services/inlineCompletionsService.ts` 已建立 editor 级 snooze 生命周期；provider 收集仍由职责不同的 `inlineCompletionProviderService.ts` 持有，inline completion contribution 通过 capability 显式连接两者。
- `browser/config/editorConfiguration.ts` 已成为生产 `View` 使用的浏览器配置 owner；此前塞在 `view.ts` 内的私有配置类已删除，common 只保留 `IEditorConfiguration` 契约。option ID 事件和 layout height 更新有直接测试；完整上游环境计算仍随 `View` / `CodeEditorWidget` 继续收口。

## 基础 owner 阻塞

同名差异表只统计“两边已经存在同路径声明”的项目，不包含上游存在而 Zeta 尚未建立的文件。架构测试当前明确暴露了这些基础缺口：

- `CursorContext`、`CursorConfiguration`、`ICursorSimpleModel` 与投影坐标转换已经存在并有直接测试；当前 `CursorsController` 仍早于 `ViewModelLines` 创建，尚未由统一 `ViewModel` 持有，因此 context 还没有进入完整生产生命周期。
- `browser/view/domLineBreaksComputer.ts` 已承担浏览器测量和批量换行请求，`common/modelLineProjectionData.ts` 拥有通用契约；注入文本装饰仍需由 `TextModel` 的 decoration owner 接入。
- `common/services/semanticTokensStylingService.ts` 已承担按 provider 缓存 styling 的服务生命周期，`semanticTokensProviderStyling.ts` 负责单 provider 解析；本地 provider 的 `LanguageToken` 表示与上游 legend metadata 仍不同。
- `browser/editorBrowser.ts` 还没有完整的 `ICodeEditor` / `IDiffEditor` 契约，因此稳定滚动、编辑器服务和多个 contrib 仍不能按上游调用链收口。
- `common/services/resolverService.ts` 已恢复 URI text-model resolver 契约，`platform/editor/common/editor.ts` 提供其异步 model 生命周期；它不承担 Workbench 的脏状态、保存、revert 和冲突处理，那些职责继续留在独立的 `ITextModelResourceService`。
- 架构必需文件检查现在一次报告全部缺口；只剩依赖统一 `ViewModel` rendering/decorations 查询的 `common/viewLayout/viewLinesViewportData.ts`。browser `EditorConfiguration`、resolver service、worker service 与 worker 入口已经建立。
- 上游 `IClipboardCopyEvent` / `createClipboardCopyEvent` 依赖 ViewModel 生成 `dataToCopy` 并负责写入编辑器元数据；本地现有 DOM 包装已改为 `IEditorClipboardCopyEvent` / `createEditorClipboardCopyEvent`，不再冒充该能力。

## 验证状态

- 本轮 cursor movement、delete、word、`SelectionSet` 输入事务、键盘/折行导航、公共入口与架构联合回归共 64 项，其中 63 项通过；唯一失败是下述 `ViewportData` owner 缺口。`Cursor` / `CursorCollection` 的 marker、primary-first、overlap normalize 和 model/view state 另有直接测试。
- `tsconfig.test.json` 的 editor 范围已无类型错误；全仓仍被 Workbench 调用端漂移阻塞。
- Editor 架构测试仍有 1 项失败，明确只列出 `ViewportData` 这个真实 owner 缺口；resolver service、browser EditorConfiguration、worker service 与 worker 入口已经通过。公共入口和 standalone language provider API 回归已经通过。
- 118 项账目当前为 106 项已处理、12 项待处理。下一批先让统一 `ViewModel` 持有 `ViewModelLines`、坐标转换和 `CursorsController`，再收口其余 cursor 类以及 `View` / `CodeEditorWidget`。
