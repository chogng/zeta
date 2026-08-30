# Editor API 对齐状态

> 本表记录 2026-08-30 对 `zeta-ts/src/zeta/editor` 生产 TypeScript 文件的扫描结果。分层依据为 VS Code 的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 和仓库内 `vscode-api-alignment` skill。

## 当前结论

- 2026-08-30 重新按相对路径扫描非测试 `.ts`、`.tsx`、`.js`、`.css` 生产文件：Zeta Editor 568 个，VS Code Editor 727 个；287 个同路径，281 个仅本地，440 个仅上游。
- 首次重扫发现 49 个目录大小写错误，全部来自工作区实际目录 `browser/viewparts` 与上游 `browser/viewParts` 不一致；已做两步大小写重命名，当前大小写错误为 0。
- 账目摘要：初始确认 118 组同名声明结构差异，已处理 7 组，剩余 111 组。原先标成已处理的 106 组已经逐项重查；只有 7 个声明通过文件集合、import owner、生产调用链和生命周期复核，其余全部退回待处理表。
- 281 个仅本地文件正在逐项分类为“错误承载，迁移并删除”或“Zeta 专有”。分类完成前，不再声称不存在 import owner、重复 owner 或错放文件问题，也不新增 Editor 生产文件。
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
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardPasteEvent` | 字段、构造行为和外部数据转换与上游一致；生产调用经过输入上下文、Clipboard contribution 和 Observable Editor，浏览器测试覆盖 metadata 与外部数据转换 |
| `common/cursor/cursorColumnSelection.ts` | `ColumnSelection` | 同路径实现与上游归一化文本一致；生产鼠标列选经过 `MouseHandler`、`CursorConfiguration`、视觉行模型和坐标转换，直接测试覆盖方向与短行行为 |
| `contrib/colorPicker/browser/colorPickerModel.ts` | `ColorPickerModel` | 公共成员、颜色与 presentation 事件、切换和释放生命周期与上游一致；生产由 Color Picker controller 创建并由 dialog 消费 |
| `standalone/browser/standaloneEditor.ts` | `createModel`、`getModel`、`getModels`、`setModelLanguage` | 公共模型边界使用 `ITextModel`；`createModel` 委托 `standaloneCodeEditor.ts::createTextModel`，未显式给语言时按 URI 和首行推断，显式语言优先；模型注册、查询、语言事件和释放由 Standalone 测试覆盖 |

## 尚未补齐的同名契约

下表第一部分保留此前的处理说明，便于追查错误判断；这些结论已经撤回，表内所有声明都需要按对应 owner 切片继续迁移。三个 Render Strategy 的路径已从错误的 `gpu/raster` 修正为上游实际的 `gpu/renderStrategy`。

| 文件 | 声明 | 此前结论（已撤回） |
| --- | --- | --- |
| `browser/controller/dragScrolling.ts` | `DragScrolling` | 当前只有仅本地 `bidirectionalDragScrolling.ts`，其双轴像素滚动职责不同于上游抽象 owner；需随 `ViewContext`、outside-editor target、`MouseTargetFactory`、render/hit-test、RTL 与 `dispatchMouse` 整链迁移后删除旧文件 |
| `browser/controller/mouseHandler.ts` | `MouseHandler` | 本地 pointer/selection owner 改为 `EditorPointerSelectionHandler` |
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardCopyEvent` | 本地职责已改名或移出上游 owner |
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
| `browser/gpu/renderStrategy/baseRenderStrategy.ts` | `BaseRenderStrategy` | 本地职责已改名或移出上游 owner |
| `browser/gpu/renderStrategy/fullFileRenderStrategy.ts` | `FullFileRenderStrategy` | 本地职责已改名或移出上游 owner |
| `browser/gpu/renderStrategy/viewportRenderStrategy.ts` | `ViewportRenderStrategy` | 本地职责已改名或移出上游 owner |
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
| `common/cursor/cursorMoveOperations.ts` | `MoveOperations` | 文件正文与上游一致，但生产键盘导航仍走仅本地的 `cursorNavigation.ts`，尚未形成 `CursorMoveCommands → MoveOperations` 调用链 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | `PieceTreeTextBuffer` | 已恢复 `common/model.ts` 的 `ITextBuffer` owner、1-based Position/Range 查询、原子编辑与逆编辑、逐行搜索、内容事件、BOM/EOL 和释放契约；红黑树内部结构保持本地实现 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts` | `PieceTreeTextBufferBuilder` | 已按上游两阶段 builder/factory 契约对齐 |
| `common/services/model.ts` | `IModelService` | 已恢复 `ITextModel`、buffer factory、creation options 与 edit source 契约 |
| `common/services/modelService.ts` | `ModelService` | 模型创建配置改由 `platform/configuration` 注入；语言/资源配置失效会更新现有模型，EOL 由 `ITextResourcePropertiesService` 决定，关闭文件的 undo/redo 以 SHA-1 校验内容并受内存上限约束 |
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
| `standalone/browser/standaloneEditor.ts` | `getEditors` | 改由 `IWidgetCodeEditorRegistry` 统一持有并返回实际编辑器对象 |
| `standalone/browser/standaloneEditor.ts` | `create` | 返回 `standaloneCodeEditor.ts` 的 `StandaloneEditor`；`create()`、`onDidCreateEditor()` 与 `getEditors()` 现在共享同一对象身份和释放时机，不再把内部 widget 当成另一个编辑器暴露 |

### 原待处理项

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
| `common/cursor/cursorWordOperations.ts` | `WordOperations` | 文件正文与上游一致，鼠标选词已接入；按词删除仍由仅本地 `selectionSetWordOperations.ts` 承担。历史 `wordSelection.ts` 已删除且无残留 import；仍需随统一 `CursorsController` 接入生产编辑链 |
| `common/cursor/oneCursor.ts` | `Cursor` | 成员边界、`modelState` / `viewState`、tracked range 与折行坐标转换已与上游一致；仍需随统一 `ViewModel` 进入生产生命周期 |
| `common/model/textModel.ts` | `TextModel` | 文本模型与 Piece Tree |
| `common/model.ts` | `ITextModel` | 文本模型与 Piece Tree |

## 当前已验证能力

- 118 项严格完成项只有上表 4 行、7 个声明；“类名已改名”“成员数量相同”或“本地实现能工作”都不再作为完成依据。
- Standalone 模型创建现在由 `standaloneCodeEditor.ts::createTextModel` 统一决定语言：显式语言优先，否则读取 URI 与第一行；Model Service 仍是模型注册、查询、语言事件和释放 owner。
- `IClipboardPasteEvent`、`ColumnSelection`、`ColorPickerModel` 已分别通过真实输入、鼠标列选和 Color Picker 生产调用链复核。
- `cursorColumns.ts`、`base/common/charCode.ts`、`base/common/uint.ts` 已作为后续 Cursor 迁移的同路径基础能力落地；它们不计入 118 项完成数。
- TextModel 现在唯一持有 decoration range，`deltaDecorations`、owner 隔离、编辑后的 tracked range、lane 失效和 collection 释放已通过 29 项定向测试；`ITextModel` 仍缺 tokenization、bracket pairs 与内部事件链，因此本项不计入 118 项完成数。

## 待处理 owner 顺序

| 顺序 | 所有权切片 | 当前问题 | 闭环条件 |
| --- | --- | --- | --- |
| 1 | Platform 配置与语言身份 | 配置 override 事件、全局 Registry、Modes Registry、语言实例 Registry 和语言配置 Registry 未形成上游链；现有语言配置服务有 28 个生产调用方 | 先统一配置键、override 与 Registry，再迁移语言身份和语言配置调用方，删除旧 owner |
| 2 | TextModel parts | decoration range 已收回 TextModel；token 状态仍由每个 Editor 持有，tokenization、bracket pairs、内部内容事件和 ViewModel 注册仍未形成上游 owner 链 | TextModel 唯一持有 tokenization、bracket pairs 与 decoration parts；Model Service、Piece Tree、undo/redo 构造与测试同步完成 |
| 3 | ViewModel 与 Cursor | `CursorsController` 仍使用 `SelectionSet + EditorEditCommand`，并早于 `ViewModelLines` 创建；Cursor 目录 7 个仅本地文件仍承载上游职责 | 建立 `ViewModelImpl → CursorsController → CursorCollection → CommandExecutor` 唯一链，迁移调用方后删除 7 个旧文件 |
| 4 | ViewContext、ViewPart 与 View | 23 个同路径 View Part 被本地 scheduler 类占用，事件、render 阶段和释放由手工 coordinator 调度 | 先恢复 ViewContext 事件与 ViewPart 生命周期，再迁移 View 和全部 Part；不能逐个复制叶子类 |
| 5 | CodeEditor Widget 与服务 | `CodeEditorWidget`、`ICodeEditor`、编辑器服务和 contribution 生命周期不完整；Workbench 仍导入缺失的 Diff/MultiDiff canonical export | Widget、服务、贡献初始化、model attach/detach、view state 和公开对象身份同批闭环 |
| 6 | GPU 与 Editor contribution | Styled GPU 是一条独立生产链；19 个 contribution 通过改成 `Editor*` 隐藏同路径声明缺口 | GPU 按 atlas→rasterizer→strategy→context→ViewLinesGpu 整链迁移；contribution 随各自 Widget/服务 owner 迁移 |

## 验证状态

- 文件集合审计：287 个同路径、0 个大小写错误、281 个仅本地、440 个仅上游；Zeta 568 个生产文件，VS Code 727 个。该结果只说明路径集合，不说明同路径文件的职责和 API 已一致。
- 118 项账本校验通过：7 项已处理、111 项待处理、总计 118 个唯一声明。
- `tsconfig.stanza.json --noEmit` 通过；严格已处理项定向测试 20/20 通过，其中 Standalone 13 项，Clipboard、Column Selection 与 Color Picker Model 7 项。TextModel decoration 前置另有 29/29 项定向测试通过，但尚未计入 118 项完成数。
- `tsconfig.test.json` 仍报告 11 个已有错误，集中在 Workbench Dialog、Diff/MultiDiff canonical export 与 PDF getter 调用；本次 Standalone 文件没有新增类型错误。
- 下一批按上表 owner 顺序推进；只有完成生产调用方迁移、删除旧入口并通过相关测试后，才会从 111 项中扣减。
