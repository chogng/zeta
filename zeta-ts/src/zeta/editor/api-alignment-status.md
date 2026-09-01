# Editor API 对齐状态

> 本表记录 2026-08-30 对 `zeta-ts/src/zeta/editor` 生产 TypeScript 文件的扫描结果。分层依据为 VS Code 的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 和仓库内 `vscode-api-alignment` skill。

## 当前结论

- 2026-08-31 重新按相对路径扫描非测试 `.ts`、`.tsx`、`.js`、`.css` 生产文件：Zeta Editor 590 个，VS Code Editor 729 个；381 个同路径，209 个仅本地，348 个仅上游。
- 首次重扫发现 49 个目录大小写错误，全部来自工作区实际目录 `browser/viewparts` 与上游 `browser/viewParts` 不一致；已做两步大小写重命名，当前大小写错误为 0。
- 账目摘要：初始确认 118 组同名声明结构差异，已处理 31 组，剩余 87 组。只有通过文件集合、import owner、生产调用链和生命周期复核的声明才计入已处理。
- 209 个仅本地文件正在逐项分类为“错误承载，迁移并删除”或“Zeta 专有”。分类完成前，不再声称不存在 import owner、重复 owner 或错放文件问题，也不新增 Editor 生产文件。
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
| `browser/controller/editContext/clipboardUtils.ts` | `IClipboardCopyEvent` | 公开成员与上游归零；事件在输入上下文中生成选区文本、来源范围、富文本和内存元数据，Clipboard contribution 直接消费该事件，浏览器测试覆盖复制、剪切、多选区、整行和系统剪贴板回退 |
| `browser/controller/editContext/clipboardUtils.ts` | `createClipboardCopyEvent` | 五参数入口与上游一致；由 `ViewContext` 读取配置和选区并负责标准剪贴板数据及元数据写入，旧的无模型事件入口已删除，生产调用只经过两个输入实现 |
| `browser/services/codeEditorService.ts` | `ICodeEditorOpenHandler` | 由 `AbstractCodeEditorService` 按新注册优先顺序调用，首个返回编辑器的处理器终止链路；单项释放只移除对应处理器，测试覆盖继续查找、短路和释放 |
| `browser/services/codeEditorService.ts` | `ICodeEditorService` | 公共成员差异归零；代码与差异编辑器的创建、加入和移除由各 Widget 的真实生命周期触发，打开处理器、资源模型属性、临时模型属性、装饰类型和当前编辑器均由同一浏览器服务提供，Workbench 差异窗格与快速差异视图使用同一服务实例 |
| `browser/services/abstractCodeEditorService.ts` | `AbstractCodeEditorService` | 抽象层只持有跨宿主共享的编辑器注册表、处理器链、资源属性、临时属性和装饰类型；当前编辑器由具体浏览器宿主持有；临时属性按 URI 与模型销毁释放，装饰样式按引用计数和服务生命周期释放，测试覆盖事件顺序、资源身份、父子装饰与释放 |
| `browser/stableEditorScroll.ts` | `StableEditorScrollState`、`StableEditorBottomScrollState` | 参数收敛到 `ICodeEditor`；滚动位置、内容高度、可见范围和行坐标由 `CodeEditorWidget` 持有，CodeLens 在增删 Widget 前后使用同一编辑器恢复顶部或底部锚点，测试覆盖首末可见行、光标相对位置和真实 Widget 几何 |
| `browser/observableCodeEditor.ts` | `observableCodeEditor` | 单参数入口直接接受 `ICodeEditor`，同一编辑器始终返回同一 facade，编辑器销毁时同步释放并移出缓存 |
| `browser/observableCodeEditor.ts` | `ObservableCodeEditor` | 公共成员与上游归零；模型、版本、选区、焦点、组合输入、键入、粘贴、布局、滚动、内容尺寸、装饰和 Widget 均通过 `ICodeEditor` 观察，不再读取 `CodeEditorWidget.view` 或 `viewport`，3 项测试覆盖响应式更新、行坐标、装饰所有权与销毁 |
| `browser/view/viewUserInputEvents.ts` | `ViewUserInputEvents` | 公开回调、构造参数、事件类型和静态目标转换入口与上游一致；鼠标事件由 `MouseHandler` 解析为视图坐标，经 `ViewController` 转发后在此统一转换为模型坐标，Widget 不再另建 DOM 监听链；测试覆盖普通 target、View Zone 嵌套坐标和真实 Widget 事件发布 |
| `contrib/zoneWidget/browser/zoneWidget.ts` | `ZoneWidget` | 恢复 `IOptions`、`IStyles`、`OverlayWidgetDelegate`、`ZoneWidget` 及其子类扩展点；独立实现通过 `ICodeEditor` 持有模型锚点、视图区、布局、滚动、选区与释放，Peek、Call/Type Hierarchy、跳转结果和 Quick Diff 均传递真实编辑器对象，定向测试覆盖换行锚点、布局、缩放、样式和选区保持 |
| `contrib/wordHighlighter/browser/textualHighlightProvider.ts` | `TextualMultiDocumentHighlightFeature` | 由语言能力服务统一注册单文档与多文档文本高亮 provider；多编辑器共享同一服务时按引用计数持有注册，不再维护重复的模型 target 表，provider 直接使用 `ITextModel.uri` 返回跨文档结果，Word/Selection Highlighter 7 项测试覆盖 Unicode、语义优先、多文件、取消和导航 |
| `common/cursor/cursorColumnSelection.ts` | `ColumnSelection` | 同路径实现与上游归一化文本一致；生产鼠标列选经过 `MouseHandler`、`CursorConfiguration`、视觉行模型和坐标转换，直接测试覆盖方向与短行行为 |
| `contrib/colorPicker/browser/colorPickerModel.ts` | `ColorPickerModel` | 公共成员、颜色与 presentation 事件、切换和释放生命周期与上游一致；生产由 Color Picker controller 创建并由 dialog 消费 |
| `contrib/folding/browser/foldingDecorations.ts` | `FoldingDecorationProvider` | 公共配置与装饰事务由该 provider 持有；生产链从 Folding Model 经编辑器所有者写入 TextModel，再由 Folding decoration source 交给 View 渲染，释放时只清理对应编辑器的装饰；折叠背景、占位符和控制图标颜色由主题 token 持有，测试覆盖配置、所有权、折叠状态和 DOM 输出 |
| `standalone/browser/standaloneEditor.ts` | `createModel`、`getModel`、`getModels`、`setModelLanguage` | 公共模型边界使用 `ITextModel`；`createModel` 委托 `standaloneCodeEditor.ts::createTextModel`，未显式给语言时按 URI 和首行推断，显式语言优先；模型注册、查询、语言事件和释放由 Standalone 测试覆盖 |
| `common/viewLayout/lineHeights.ts` | `CustomLineHeightData` | 构造参数、公开字段和 `fromDecorations` owner 与上游一致；生产由 `ViewModelImpl` 转换模型装饰，再交给 `LinesLayout`，测试覆盖视觉范围转换与配置行高倍率 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.ts` | `PieceTreeTextBuffer` | 实现 `common/model.ts` 的 `ITextBuffer` 契约；独立红黑树实现负责 1-based 查询、原子编辑与逆编辑、搜索、内容事件、BOM/EOL、快照和释放，确定性编辑测试同时检查字符串结果与树不变量 |
| `common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.ts` | `PieceTreeTextBufferBuilder` | 保持 Builder → Factory 两阶段 owner；跨 chunk 连接 CRLF 与代理对，按主导换行选择 EOL，并把 `finish(false)` 的保留换行语义传给主缓冲区 |
| `common/services/model.ts` | `IModelService` | 公共方法、事件、模型类型与 owner 一致；`getCreationOptions` 只暴露语言 ID、确定资源和 Widget 类型，不再把实现类的宽参数泄漏到服务接口 |
| `common/model.ts` | `ITextModel` | 同名成员差异归零；模型统一持有装饰事务和 owner 查询、外部 undo/redo 入口、token/语言配置/字体/行高事件以及 ViewModel 注册生命周期，27 项测试覆盖事务回滚、事件顺序、所有者隔离、历史负载和释放 |
| `common/services/modelService.ts` | `ModelService` | 统一持有资源模型、创建配置、语言事件、内容更新、关闭文件历史与释放；生产由 Standalone 服务注册并通过 `IModelService` 使用，5 项测试覆盖资源身份、事件、配置失效、工厂所有权和历史校验 |
| `common/viewLayout/lineHeights.ts` | `LineHeightsManager` | 由 `LinesLayout` 唯一持有默认行高和自定义行高范围；重叠范围取最大高度，插删行移动或收缩范围，5 项测试覆盖累计高度、范围变更和装饰转换 |
| `common/viewLayout/linesLayout.ts` | `LinesLayout` | 文件只保留行高、纵向几何与空白区职责；视区编排移回 `viewLayout.ts`，生产由布局 owner 调用，3 项直接测试与 11 项布局测试覆盖批处理、坐标、插删行、视图区间和空白区查询 |
| `standalone/browser/standaloneEditor.ts` | `create` | 创建并返回同一个 `StandaloneEditor` 实例；该实例由代码编辑器服务登记并触发创建事件，释放时从服务移除，仅隐式创建的模型随编辑器释放 |
| `standalone/browser/standaloneEditor.ts` | `getEditors` | 直接读取代码编辑器服务持有的当前实例；与 `create`、`onDidCreateEditor` 共享对象身份和释放时机，Standalone 测试覆盖双编辑器登记、共享模型和独立释放 |
| `browser/viewParts/viewLines/viewLineOptions.ts` | `ViewLineOptions` | 公开成员差异归零；从计算后的编辑器配置和主题类型生成不可变行渲染快照，由 `ViewLines` 持有并在配置变化时比较后通知已渲染行；段落方向、制表宽度和 GPU 输入不再错误归入该类型，定向测试覆盖全部快照字段、相等比较与调用链 |

## 尚未补齐的同名契约

下表第一部分保留此前的处理说明，便于追查错误判断；这些结论已经撤回，表内所有声明都需要按对应 owner 切片继续迁移。三个 Render Strategy 的路径已从错误的 `gpu/raster` 修正为上游实际的 `gpu/renderStrategy`。

| 文件 | 声明 | 此前结论（已撤回） |
| --- | --- | --- |
| `browser/controller/dragScrolling.ts` | `DragScrolling` | 当前只有仅本地 `bidirectionalDragScrolling.ts`，其双轴像素滚动职责不同于上游抽象 owner；需随 `ViewContext`、outside-editor target、`MouseTargetFactory`、render/hit-test、RTL 与 `dispatchMouse` 整链迁移后删除旧文件 |
| `browser/controller/mouseHandler.ts` | `MouseHandler` | 已只保留浏览器指针捕获、拖动、自动滚动、目标解析和输入事件发布；选区策略已迁入 `ViewController.dispatchMouse`，构造参数仍待随 `ViewContext` 和 pointer helper 收敛 |
| `browser/controller/editContext/native/debugEditContext.ts` | `DebugEditContext` | 本地职责已改名或移出上游 owner |
| `browser/controller/editContext/native/nativeEditContextUtils.ts` | `FocusTracker` | 已恢复公开名并由浏览器输入实现实际持有；构造契约和日志依赖仍待收敛 |
| `browser/controller/editContext/editContext.ts` | `AbstractEditContext` | 已进入 `ViewPart` 生命周期并统一剪贴板事件、输入路由和组合输入状态；抽象层仍保留 Zeta 的公共输入契约，需继续缩小与上游声明成员的差异 |
| `browser/controller/editContext/native/nativeEditContext.ts` | `NativeEditContext` | 已由 `View` 持有，接入 `ViewContext`、视图事件、预渲染几何读取、渲染写入、按编辑器 ID 注册及跨 document 重新挂接；浏览器缓冲区和辅助阅读器的成员契约仍待收敛 |
| `browser/controller/editContext/native/screenReaderContentRich.ts` | `RichScreenReaderContent` | 已恢复公开名并由 `ScreenReaderSupport` 实际选择；配置事件和视图渲染阶段仍待收敛 |
| `browser/controller/editContext/native/screenReaderContentSimple.ts` | `SimpleScreenReaderContent` | 已恢复公开名并实际承担简单无障碍镜像；选择同步和 `ViewContext` 构造契约仍待收敛 |
| `browser/controller/editContext/native/screenReaderSupport.ts` | `ScreenReaderSupport` | 已由 `NativeEditContext` 持有并接入配置、光标、内容、装饰、滚动、空白区及 prepare/render 生命周期；动态辅助阅读器配置和剩余成员契约仍待收敛 |
| `browser/controller/editContext/textArea/textAreaEditContext.ts` | `TextAreaEditContext` | 已由 `View` 持有，接入 `ViewContext`、视图事件、渲染阶段、按编辑器 ID 注册、`getTextAreaDomNode` 和真实输入调用链；文本窗口与辅助阅读器成员契约仍待收敛 |
| `browser/controller/editContext/textArea/textAreaEditContextInput.ts` | `TextAreaInput` | 已恢复公开名并由 `focusTextArea`、DOM 事件、选区和释放形成真实调用链；host 事件契约仍待收敛 |
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
| `browser/viewParts/overlayWidgets/overlayWidgets.ts` | `ViewOverlayWidgets` | 已恢复公开名、`ViewPart` 生命周期、DOM owner、布局缓存和 widget 生命周期；剩余成员差异继续按真实调用方收敛 |
| `browser/view/viewOverlays.ts` | `ViewOverlays` | 已恢复 `ViewPart` owner、`addDynamicOverlay`、统一可见行 DOM 和 `ContentViewOverlays` / `MarginViewOverlays`；逐行覆盖层不再各建一套行容器 |
| `browser/view/dynamicViewOverlay.ts` | `DynamicViewOverlay` | 已继承 `ViewEventHandler`，渲染契约改为 `prepareRender(context)` 后按行号返回片段；不再继承 Part 或直接持有编辑器 DOM |
| `browser/view/viewController.ts` | `ViewController` | 已只负责命令、组合输入协调、焦点调用和 `dispatchMouse` 选区策略；输入 Part 的创建、渲染与释放已迁入 `View`，剪贴板委托和剩余公开成员仍待收敛 |
| `browser/viewParts/gpuMark/gpuMark.ts` | `GpuMarkOverlay` | 本地职责已改名或移出上游 owner |
| `browser/viewParts/rulersGpu/rulersGpu.ts` | `RulersGpu` | 本地职责已改名或移出上游 owner |
| `browser/viewParts/blockDecorations/blockDecorations.ts` | `BlockDecorations` | 已作为独立 `ViewPart` 持有块级 DOM，不再混入逐行覆盖层 |
| `browser/viewParts/contentWidgets/contentWidgets.ts` | `ViewContentWidgets` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/currentLineHighlight/currentLineHighlight.ts` | `CurrentLineHighlightOverlay` | 已恢复逐行片段职责并由 `ContentViewOverlays` 持有 |
| `browser/viewParts/decorations/decorations.ts` | `DecorationsOverlay` | 已恢复逐行片段职责、装饰快照和失效事件 owner，并由 `ContentViewOverlays` 持有 |
| `browser/viewParts/editorScrollbar/editorScrollbar.ts` | `EditorScrollbar` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/glyphMargin/glyphMargin.ts` | `GlyphMarginWidgets` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/indentGuides/indentGuides.ts` | `IndentGuidesOverlay` | 已恢复逐行片段职责并由 `ContentViewOverlays` 持有 |
| `browser/viewParts/lineNumbers/lineNumbers.ts` | `LineNumbersOverlay` | 已从独立 Part 改为 margin 逐行覆盖层，并由 `MarginViewOverlays` 持有 |
| `browser/viewParts/linesDecorations/linesDecorations.ts` | `LinesDecorationsOverlay` | 已恢复逐行片段职责并由 `MarginViewOverlays` 持有 |
| `browser/viewParts/margin/margin.ts` | `Margin` | 已恢复公开名和 `ViewPart` 生命周期，并持有 margin 覆盖层、视图区与 glyph margin DOM |
| `browser/viewParts/marginDecorations/marginDecorations.ts` | `MarginViewLineDecorationsOverlay` | 已恢复逐行片段职责并由 `MarginViewOverlays` 持有 |
| `browser/viewParts/minimap/minimap.ts` | `Minimap` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/overviewRuler/decorationsOverviewRuler.ts` | `DecorationsOverviewRuler` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/overviewRuler/overviewRuler.ts` | `OverviewRuler` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/rulers/rulers.ts` | `Rulers` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/scrollDecoration/scrollDecoration.ts` | `ScrollDecorationViewPart` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/selections/selections.ts` | `SelectionsOverlay` | 已恢复逐行片段职责并由 `ContentViewOverlays` 持有 |
| `browser/viewParts/viewCursors/viewCursor.ts` | `ViewCursor` | 已恢复公开名并由 `ViewCursors` 独立持有 |
| `browser/viewParts/viewCursors/viewCursors.ts` | `ViewCursors` | 已从逐行覆盖层移回独立 `ViewPart`，持有光标 DOM、组合输入显示和移动动画 |
| `browser/viewParts/viewLines/viewLine.ts` | `ViewLine` | 本地 scheduler 实现改为 `ViewLine` |
| `browser/viewParts/viewLines/viewLines.ts` | `ViewLines` | 已实现 `IViewLines`，DOM 行按逐行 `LineVisibleRanges` 与 `HorizontalPosition` 提供真实浏览器几何；GPU 接管的行不再触发 DOM 几何读取 |
| `browser/viewParts/viewZones/viewZones.ts` | `ViewZones` | 已恢复公开名并接入 `ViewPart` 事件与释放链 |
| `browser/viewParts/whitespace/whitespace.ts` | `WhitespaceOverlay` | 已恢复逐行片段职责并由 `ContentViewOverlays` 持有 |
| `browser/widget/codeEditor/codeEditorContributions.ts` | `CodeEditorContributions` | 本地多 context contribution owner 改为 `WidgetContributionCollection` |
| `browser/widget/diffEditor/diffEditorWidget.ts` | `DiffEditorWidget` | 本地只读虚拟化审阅面板改为 `EditorDiffWidget` |
| `browser/widget/multiDiffEditor/multiDiffEditorWidget.ts` | `MultiDiffEditorWidget` | 本地多文件审阅面板改为 `EditorMultiDiffWidget` |
| `browser/viewParts/viewLinesGpu/viewLinesGpu.ts` | `ViewLinesGpu` | 已接入 `ViewPart` 事件、渲染和释放链，并由实际绘制帧实现 `IViewLines`、行宽与横坐标命中；初始化和渲染阶段的内部 owner 仍待继续收敛 |
| `common/services/languageService.ts` | `LanguageService` | 已按上游契约对齐 |
| `common/services/languageFeatures.ts` | `ILanguageFeaturesService` | 本地 provider 集合契约改为 `IEditorLanguageFeaturesService` |
| `common/services/languageFeaturesService.ts` | `LanguageFeaturesService` | 本地 provider registry 改为 `EditorLanguageFeaturesService` |
| `common/viewLayout/viewLayout.ts` | `ViewLayout` | 已恢复标准构造入口、配置更新和 `Scrollable` 所有权；生产由唯一 `ViewModel` 持有，内容/滚动尺寸、当前视口、行坐标、可见行、空白区、特殊行高以及滚动/内容尺寸事件均由它发布；`View` 只读取布局结果，不再创建或更新第二份布局 |
| `common/cursor/cursorTypeEditOperations.ts` | `TypeWithoutInterceptorsOperation` | 已恢复上游公开名与成员边界；selection offset 归并留在文件私有 helper，不再伪装成 class API |
| `common/cursor/cursorTypeEditOperations.ts` | `AutoClosingOvertypeOperation` | 已恢复上游公开名与 `_runAutoClosingOvertype` 内部阶段，现有多光标 overtype 行为保持不变 |
| `common/cursor/cursorMoveOperations.ts` | `MoveOperations` | 文件正文与上游一致，但生产键盘导航仍走仅本地的 `cursorNavigation.ts`，尚未形成 `CursorMoveCommands → MoveOperations` 调用链 |
| `contrib/colorPicker/browser/colorPickerWidget.ts` | `ColorPickerWidget` | 本地职责已改名或移出上游 owner |
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

### 原待处理项

| 文件 | 声明 | 分类 |
| --- | --- | --- |
| `browser/view.ts` | `View` | 已统一持有 `ViewController` 和输入 Part，并把输入纳入视图挂载、渲染、几何更新与释放链；其余视图部件和公开成员仍待逐项收敛 |
| `browser/widget/codeEditor/codeEditorWidget.ts` | `CodeEditorWidget` | 浏览器编辑器契约 |
| `common/cursor/cursor.ts` | `CursorsController` | 已恢复上游公开名；ViewModel 与 cursor context 调用链尚未补齐 |
| `common/cursor/cursorCollection.ts` | `CursorCollection` | 成员边界、primary-first 状态、marker 生命周期和 overlap normalize 已与上游一致；仍需随统一 `ViewModel` 接入生产 `CursorsController` |
| `common/cursor/cursorDeleteOperations.ts` | `DeleteOperations` | 成员边界比较为 0，已恢复 `CursorConfiguration`、`Selection[]`、`ICommand` 和自动闭合范围语义；本地 `SelectionSet` 事务位于 `selectionSetDeleteOperations.ts`。仍需随统一 `CursorsController` 接入生产编辑链 |
| `common/cursor/cursorMoveCommands.ts` | `CursorMoveCommands` | 已恢复上游公开名；命令参数与 ViewModel 调用链仍需核对 |
| `common/cursor/cursorTypeOperations.ts` | `TypeOperations` | 已恢复上游公开名；输入策略仍需逐项核对 |
| `common/cursor/cursorWordOperations.ts` | `WordOperations` | 文件正文与上游一致，鼠标选词已接入；按词删除仍由仅本地 `selectionSetWordOperations.ts` 承担。历史 `wordSelection.ts` 已删除且无残留 import；仍需随统一 `CursorsController` 接入生产编辑链 |
| `common/cursor/oneCursor.ts` | `Cursor` | 成员边界、`modelState` / `viewState`、tracked range 与折行坐标转换已与上游一致；仍需随统一 `ViewModel` 进入生产生命周期 |
| `common/model/textModel.ts` | `TextModel` | 文本模型与 Piece Tree |

## 当前已验证能力

- 118 项严格完成项为上表 18 行、22 个声明；“类名已改名”“成员数量相同”或“本地实现能工作”都不作为完成依据。
- Standalone 模型创建现在由 `standaloneCodeEditor.ts::createTextModel` 统一决定语言：显式语言优先，否则读取 URI 与第一行；Model Service 仍是模型注册、查询、语言事件和释放 owner。
- `IClipboardPasteEvent`、`ColumnSelection`、`ColorPickerModel` 已分别通过真实输入、鼠标列选和 Color Picker 生产调用链复核。
- `cursorColumns.ts`、`base/common/charCode.ts`、`base/common/uint.ts` 已作为后续 Cursor 迁移的同路径基础能力落地；它们不计入 118 项完成数。
- `ITextModel` 的公开成员、内部历史入口和 ViewModel 生命周期已对齐；`TextModel` 现在唯一持有 decoration range、owner 隔离、模型部件事件与 tokenization/bracket pairs 调度。实现类仍需继续统一私有 owner，因此 `TextModel` 声明本身尚未计入完成数。

## 待处理 owner 顺序

| 顺序 | 所有权切片 | 当前问题 | 闭环条件 |
| --- | --- | --- | --- |
| 1 | Platform 配置与语言身份 | 配置 override 事件、全局 Registry、Modes Registry、语言实例 Registry 和语言配置 Registry 未形成上游链；现有语言配置服务有 28 个生产调用方 | 先统一配置键、override 与 Registry，再迁移语言身份和语言配置调用方，删除旧 owner |
| 2 | TextModel parts | `ITextModel` 成员、模型部件事件和 ViewModel 注册链已闭合；实现类仍保留 Zeta 文档块、行身份与历史能力，并与上游私有阶段存在差异 | 明确这些 Zeta 能力在同一 TextModel 内的长期边界，继续统一基础模型私有 owner，不为私有常量或字段制造同名壳 |
| 3 | ViewModel 与 Cursor | 生产构造链已收敛为 `CodeEditorWidget → ViewModel → View`；行映射、坐标转换、光标、布局、装饰和事件只有一份，`ViewModelLinesFromProjectedModel` 只由 `ViewModel` 创建。当前缺口是输入与 contribution 仍通过内部光标执行器工作，Widget 还保留获取该执行器的内部入口 | 将选择、输入、组合输入、命令执行和只读事件逐项改为 `ViewModel` 契约，删除内部执行器入口；相关调用方完成迁移后再把本切片计为完成 |
| 4 | ViewContext、ViewPart 与 View | `ViewContext → ViewPart → View` 生命周期已经接通，内容/margin 覆盖层统一逐行 DOM，块装饰和光标回到独立 Part；标准渲染上下文与 DOM/GPU `IViewLines` 几何已接通。当前缺口只剩两个输入实现尚未进入同一 Part 渲染阶段 | 迁移输入 Part，不保留第二套调度框架 |
| 5 | CodeEditor Widget 与服务 | `CodeEditorWidget`、`ICodeEditor`、编辑器服务和 contribution 生命周期不完整；Workbench 仍导入缺失的 Diff/MultiDiff canonical export | Widget、服务、贡献初始化、model attach/detach、view state 和公开对象身份同批闭环 |
| 6 | GPU 与 Editor contribution | Styled GPU 是一条独立生产链；19 个 contribution 通过改成 `Editor*` 隐藏同路径声明缺口 | GPU 按 atlas→rasterizer→strategy→context→ViewLinesGpu 整链迁移；contribution 随各自 Widget/服务 owner 迁移 |

## 验证状态

- 文件集合审计：381 个同路径、0 个大小写错误、209 个仅本地、348 个仅上游；Zeta 590 个生产文件，VS Code 729 个。该结果只说明路径集合，不说明同路径文件的职责和 API 已一致。
- 118 项账本校验通过：31 项已处理、87 项待处理、总计 118 个唯一声明。
- `tsconfig.stanza.json --noEmit` 与 `tsconfig.extensions-test.json --noEmit` 通过；渲染/GPU、指针命中和 Widget 的 26 个相关用例通过。整个 `editor` 批跑没有行为断言失败，但一个单独运行可通过的指针命中用例被测试运行器报告事件循环提前结束，仍需继续定位该批跑残留句柄。完整测试类型检查仍只有 Sessions 与 Workbench Chat 的 9 个既有错误；生产 Widget 已接入唯一 ViewModel，但输入与 contribution 尚未移除内部光标入口，因此当前不从 118 项账目中扣减。
- `tsconfig.test.json` 仍报告 9 个已有错误，集中在 Sessions 与 Workbench Chat；本轮 Editor 文件没有新增类型错误。`ViewLineOptions`、`ViewLine` 与 GPU 的 9 项定向测试通过。
- 下一批按上表 owner 顺序推进；只有完成生产调用方迁移、删除旧入口并通过相关测试后，才会从 87 项中扣减。
