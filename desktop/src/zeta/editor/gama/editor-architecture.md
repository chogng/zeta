# Gama Editor：结构化文档编辑器的文件级架构

> Gama 是 `editor` 下与 Alpha 并列的结构化编辑器域。它沿用 input → pane → editor → model reference → widget 骨架，以树形 `DocumentModel` 取代行式 `TextModel`。行为契约、测试和限制见 [`README.md`](./README.md)。

`textBlock` 是 Gama 的普通文本块；Alpha 的 `codeBlock` 是 Alpha 的代码语义。`TextEditorWidget` 只代表一个 `textBlock` 中的嵌入式行编辑器，它包裹 Alpha 的 `CodeEditorWidget`，不再是 Gama 的 pane 或目录名。

## 与 Alpha 的同构关系

| Alpha | Gama | 共同职责 | 领域差异 |
| --- | --- | --- | --- |
| `browser/editorInput.ts` | `browser/editorInput.ts` | 稳定 editor ID 与 Workbench input 匹配 | Gama 按 Academic profile 匹配结构化资源 |
| `EditorPane` | `EditorPane` | `IEditorPane` 生命周期、布局、可见性与 working copy 暴露 | Gama 创建 document-model service |
| Alpha 主编辑器（`CodeEditorWidget`） | `EditorWidget` | editor DOM projection、focus 与命令输入 | Gama 投影树节点、marks、node views 与结构化 selection |
| `BrowserTextModelService` | `BrowserDocumentModelService` | 解析资源、形成模型引用、保存/恢复/外部变更生命周期 | Gama 反序列化 schema 校验的 document envelope，纯文本迁移为段落 |
| `CodeEditorWidget` | `TextEditorWidget` | 可聚焦的行式编辑器 widget | Gama 仅在 `textBlock` 中嵌入 Alpha widget；它不拥有 Gama document identity |

## 目录规范

目录以本仓库 `../vscode/src/vs/editor` 的实际结构为准：`common/{core,model,services}`、`browser/{widget,services,media}`、`contrib/<feature>/{common,browser}`、`test/{common,browser}`。

```text
editor/gama/
  editor.api.ts                    # DOM-free structured-document API
  editor.all.ts                    # public contribution bundle for product entries
  editor.main.ts                   # editor.all + editor.api
  common/
    core/                         # position、selection
    model/                        # document、schema、transaction、history、serialization
    commands/                     # DOM-free 文档命令
    services/documentModelService # document model reference contract
  browser/
    editorInput.ts                # GAMA_EDITOR_ID 与 resource matching
    editorPane.ts             # IEditorPane host
    editorWidget.ts                 # structured editor + DOM projection
    services/
      browserDocumentModelService # runtime document reference factory
      documentWorkingCopy         # persistence / dirty / external-change adapter
      editorProfile           # profile → pane options
    widget/
      textEditorWidget            # one textBlock's embedded Alpha line editor
      documentOutlineNavigator
    media/editorWidget.css
  contrib/
    academic/{common,browser}     # schema、node views、pane contribution
    citation/{common,browser}
    collaboration/common
  test/{common,browser}
```

`editor/` 根目录只暴露 `alpha/` 与 `gama/`；不得重新引入根级 `common/`、`core/` 或 `textEditorWidget/`。跨 editor 的中性资源语言识别由 `platform/language/common` 所有。

## 职责与依赖

| 层 | 允许依赖 | Owner |
| --- | --- | --- |
| `gama/common` | `base/common` | immutable document、schema、transaction、history、selection、serialization 与 `IDocumentModelService` contract；不得依赖 Alpha、Workbench、Electron 或 DOM |
| `browser/services/browserDocumentModelService` | Gama common、Workbench text/working-copy contracts | document reference 构造与释放；不得投影 DOM 或解释 Academic/Citation schema |
| `browser/editorPane` | browser services、Workbench editor contracts | pane host、editor 生命周期、layout、visibility、working-copy bridge |
| `browser/editorWidget` | Gama common、document-model service、DOM、embedded-editor contract | 一个 document reference 的 node/mark projection、input、outline 与 focus；不得注册产品 pane |
| `browser/widget/textEditorWidget` | `IEmbeddedTextEditorFactory` | 一个 `textBlock` 的 Alpha line-editor wrapper；不得创建/保存整个 Gama document |
| `contrib/formatting/browser` | Gama browser、base toolbar | Word-like formatting toolbar；只通过 Gama editor command/selection seam 工作，不依赖 Workbench 或产品 composition |
| `contrib/<feature>/common` | Gama common | feature 的 schema、commands、collaboration data |
| `contrib/<feature>/browser` | Gama browser、feature common、Workbench composition contracts | node views、toolbar、profile 与该 feature 的 pane registration |

Gama 的对外入口分工镜像 VS Code：`editor.api.ts` 只公开 DOM-free document model、schema、transaction 与 serialization；`editor.all.ts` 是 product entry 使用的 contribution bundle；`editor.main.ts` 是 all + api 的完整入口。当前 Gama 没有浏览器 Worker consumer，因此不提供虚假的 `editor.worker.start.ts`；未来 collaboration/layout worker 出现时才在此添加其真实启动协议。产品的 Workbench composition 在 editor domain 外按版本选择 Alpha 或 Gama entry；它不参与 EditorWidget、formatting contrib 或 document selection 的运行时。`academicEditor.contribution.ts` 在唯一的 composition seam 注入 `EmbeddedTextEditorFactory`。Alpha 从不依赖 Gama document types。
