# Aster Document Engine：结构化文档编辑器的文件级架构

> Document Engine 是扁平 Aster `editor` 域中的结构化文档同步权威。Editor 域拥有 model → widget 的编辑语义，Workbench 域用 input → pane → model reference 宿主骨架接入它；树形 `DocumentModel` 区别于行式 `TextModel`。统一目录与装配边界见 [`README.md`](./README.md)，行为契约、测试和限制见 [`document-engine.md`](./document-engine.md)。

`textBlock` 是 Document Engine 的普通文本块；`codeBlock` 是 Text Engine 的代码语义。`TextEditorWidget` 只代表一个 `textBlock` 中的嵌入式行编辑器，它包裹 Aster 的 `CodeEditorWidget`，不再代表 document pane 或独立目录。

## 与 Text Engine 的同构关系

| Text Engine | Document Engine | 共同职责 | 领域差异 |
| --- | --- | --- | --- |
| `workbench/contrib/codeEditor/browser/codeEditorInput.ts` | `workbench/contrib/documentEditor/browser/documentEditorInput.ts` | 稳定 editor ID 与 Workbench input 匹配 | Document Workbench adapter 按 Academic profile 匹配结构化资源 |
| `CodeEditorPane` | `DocumentEditorPane` | `IEditorPane` 生命周期、布局、可见性与 working copy 暴露 | Document Engine 创建 document-model service |
| `CodeEditorWidget` | `EditorWidget` | editor DOM projection、focus 与命令输入 | Document Engine 投影树节点、marks、node views 与结构化 selection |
| `BrowserTextModelService` | `BrowserDocumentModelService` | 解析资源、形成模型引用、保存/恢复/外部变更生命周期 | Document Engine 反序列化 schema 校验的 document envelope，纯文本迁移为段落 |
| `CodeEditorWidget` | `TextEditorWidget` | 可聚焦的行式编辑器 widget | Document Engine 仅在 `textBlock` 中嵌入 Aster text widget；它不把 text model identity 当作 document identity |

## 目录规范

目录以本仓库 `../vscode/src/vs/editor` 的实际结构为准：`common/{core,model,services}`、`browser/{widget,services,media}`、`contrib/<feature>/{common,browser}`、`test/{common,browser}`。

```text
editor/
  editor.api.ts                    # 两个 engine 的 DOM-free API
  editor.academic.all.ts           # Academic product contribution bundle
  editor.all.ts                    # 完整 contribution bundle
  editor.worker.start.ts           # dedicated structured-worker bootstrap
  common/
    core/                         # position、selection
    model/                        # document、schema、transaction、history、serialization
    commands/                     # DOM-free 文档命令
    services/documentModelService # document model reference contract
  browser/
    editorWidget.ts              # structured editor + DOM projection
    widget/
      textEditorWidget            # one textBlock's embedded Aster line editor
      documentOutlineNavigator
    media/editorWidget.css
  contrib/
    academic/{common,browser}     # schema 与 node views
    clipboard/browser             # external HTML → validated document fragment
    citation/{common,browser}
    collaboration/{common,browser} # rebase/session/controller + toolbar contribution
  test/{common,browser}

workbench/
  contrib/documentEditor/browser # input、pane、profile materialization
  contrib/academic/browser       # Academic profile 与 product pane registration
  services/documentEditor/browser # model reference、working copy 与 persistence adapter
  services/documentCollaboration/browser # local/remote transport adapters
```

`editor/` 根目录使用 `common/`、`browser/`、`contrib/`、`test/` 和 `aster.*` 入口；不得重新引入 engine 子目录。跨 editor 的中性资源语言识别由 `platform/language/common` 所有。

## 职责与依赖

| 层 | 允许依赖 | Owner |
| --- | --- | --- |
| `common` 中的 document owners | `base/common` | immutable document、schema、transaction、history、selection、serialization 与 `IDocumentModelService` contract；不得依赖 Text Engine、Workbench、Electron 或 DOM |
| `workbench/services/documentEditor/browser/browserDocumentModelService` | Aster common、Workbench text/working-copy contracts | document reference 构造与释放；不得投影 DOM 或解释 Academic/Citation schema |
| `workbench/contrib/documentEditor/browser/documentEditorPane` | Workbench editor contracts、Aster widget | pane host、editor 生命周期、layout、visibility、working-copy bridge |
| `browser/editorWidget` | Aster common、document-model service、DOM、embedded-editor contract | 一个 document reference 的 node/mark projection、input、outline 与 focus；不得注册产品 pane |
| `browser/widget/textEditorWidget` | `IEmbeddedTextEditorFactory` | 一个 `textBlock` 的 Aster line-editor wrapper；不得创建/保存整个 Aster document |
| `contrib/formatting/browser` | Aster browser、base toolbar | Word-like formatting toolbar；只通过 Aster editor command/selection seam 工作，不依赖 Workbench 或产品 composition |
| `contrib/collaboration/common` | Document common、document collaboration service contract | canonical/in-flight/buffered state、rebase、model controller 与 transport-neutral connection；不依赖 DOM、Workbench、Electron 或 App Server DTO |
| `contrib/collaboration/browser` | Aster browser、base toolbar | create/join/leave、remote-owner invitation and member-management toolbar/status projection；只调用 `EditorWidget` 的 collaboration seam，不拥有 transaction、room ordering 或 transport |
| `workbench/services/documentCollaboration/browser/appServerDocumentCollaborationService` | Aster service contract、platform collaboration API | App Server DTO/notification adapter；不得把 protocol names or generated DTOs leak into Aster common or widget |
| `workbench/services/documentCollaboration/browser/remoteDocumentCollaborationService` | Aster service contract、Fetch | authenticated remote HTTP/long-poll adapter；只在 runtime module 中持有 transport DTO，remote URL/token 不得进入 document/model |
| `workbench/services/documentCollaboration/browser/documentCollaborationService` | Aster service contract、transport adapters | composition-local router；按显式 target 选择 local App Server 或 remote transport，不得把这项选择下沉到 common/model |
| `contrib/<feature>/common` | Aster common | feature 的 schema、commands、collaboration data |
| `contrib/<feature>/browser` | Aster browser、feature common | node views、toolbar 与可移除 editor projection；不得注册 Workbench pane |

统一入口分工镜像 VS Code：`editor.api.ts` 同时公开 DOM-free text/document model、schema、transaction 与 serialization；`editor.academic.all.ts` 是 Academic 构建模式使用的 editor capability bundle；`editor.all.ts` 是完整 capability 集，`editor.main.ts` 组合 all + api；`editor.worker.start.ts` 统一 dedicated worker 的 structured-clone port 和资源生命周期。Workbench 模式 contribution 在 Aster 域外同时选择能力 bundle 与 pane contribution。`workbench/contrib/academic/browser/academicEditor.contribution.ts` 在唯一 composition seam 注入 `EmbeddedTextEditorFactory` 并注册 pane；`TextModel` 从不依赖 document types。
