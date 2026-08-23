# Stanza

> 本文是 `zeta-ts/src/zeta/editor` 的 canonical 目录、所有权和装配入口。Stanza 类似 Monaco 在 VS Code 中的位置；`editor/` 是一个扁平领域目录，不是第二个品牌或额外架构层。行式文本与结构化文档的设计规范分别见 [`text-engine.md`](./text-engine.md) 和 [`document-engine.md`](./document-engine.md)，跨 Workbench、文件、语言服务与 App Server 的系统边界见 [`docs/editor-architecture.md`](../../../../docs/editor-architecture.md)。

## 快速理解

Stanza 采用与 VS Code `src/vs/editor` 一致的扁平职责分区：`common` 保存 DOM-free 模型与算法，`browser` 保存 DOM 投影和宿主适配，`contrib` 保存可装配能力，`test` 保存内核级回归测试。Stanza 只有一个源码域和一套公开入口；真正不同的底层对象分别由 `TextModel` 和 `DocumentModel` 表达。

| 产品或调用方式 | 加载入口 | 得到的能力 |
| --- | --- | --- |
| 标准 editor 能力 | `editor.all.ts` | 两个产品共同验证的行式编辑能力；不注册 Workbench pane |
| Code editor 能力 | `editor.code.all.ts` | 标准能力；当前没有额外 editor contribution |
| Academic editor 能力 | `editor.academic.all.ts` | 标准能力加结构化文档 engine contribution；不注册 Workbench pane |
| DOM-free 程序化调用 | `editor.api.ts` | `TextModel`、`DocumentModel`、schema、transaction 和坐标值对象；不注册 pane |
| 完整程序化入口 | `editor.main.ts` | `editor.all.ts` 与 `editor.api.ts` 的组合 |

## 核心文档

Editor 只维护三个核心入口。实现 README 可以补充局部细节，但不得复制核心规范。

| 文档 | Canonical responsibility | 不负责 |
| --- | --- | --- |
| [`README.md`](./README.md) | 扁平目录、两个 engine、依赖方向和 Workbench 模式装配 | 单个 engine 的完整行为和实现台账 |
| [`text-engine.md`](./text-engine.md) | 行式文本内核、view 架构、input、Contribution、当前状态和演进 | Workbench pane、文件协议和 App Server transport |
| [`document-engine.md`](./document-engine.md) | Schema、structured model、transaction、browser projection、profile 和 collaboration | 行式文本语义和产品 pane 生命周期 |

跨系统的 editor/file/language/Workbench/App Server 关系只在 [`docs/editor-architecture.md`](../../../../docs/editor-architecture.md) 详细说明。[`browser/README.md`](./browser/README.md) 和其他子目录 README 面向实现维护者，记录局部调用路径、DOM ownership、failure semantics 和测试影响。

## 所有权与依赖方向

| 目录 | 允许依赖 | 拥有 | 不得拥有 |
| --- | --- | --- | --- |
| `common/core` | `base/common` | 文本坐标、文档坐标、selection、纯变换算法 | DOM、Workbench service、App Server DTO |
| `common/model` | `common/core`、`base/common` | `TextModel`、`DocumentModel`、history、schema、transaction、serialization | 文件传输、浏览器 focus、产品 profile |
| `common/cursor`、`common/viewModel`、`common/viewLayout` | 文本内核与 `base/common` | 行式编辑器实例状态和纯布局投影 | DOM 和产品判断 |
| `browser` | `common`、`base/browser` 和显式前端 service contract | code/document/diff widget、输入、viewport、contribution registry 与 editor-facing runtime adapter | Workbench pane/input、文件/working-copy 生命周期、Workbench 模式选择 |
| `contrib/<feature>` | 对应 engine 的最小 contract | 可移除的编辑能力及其命令、状态和投影 | 第二套 model、产品级 `if code/academic` |
| `editor.*.all.ts` | contribution entry | 静态 editor 能力装配 | Workbench pane/input 注册、模型或功能实现 |
| `workbench/contrib/{codeEditor,documentEditor,academic}` | Editor 与 Workbench contract | pane/input、产品 profile、embedded factory 和服务接线 | 编辑事务、selection、viewport 或 feature controller |

依赖必须保持 `workbench → editor/contrib → editor/browser/common → editor/common → base` 的方向。`src/zeta/editor` 的生产代码不得反向引用 Workbench，`src/zeta/base` 也不得反向引用 editor。结构化文档通过 editor-owned `IEmbeddedTextEditorFactory` contract 请求行编辑能力，由 Workbench 提供具体 factory；`DocumentModel` 不得依赖 `TextModel`，`TextModel` 也不得依赖 document schema。

## 两个 engine

### 行式文本 engine

`TextModel` 是纯文本、版本、transaction、undo/redo、tracked range 和 snapshot 的唯一同步权威。`CodeEditorWidget` 与 `EditorPart` 投影它，但不拥有共享 model。`browser/editorContribution.ts` 保存 feature-neutral 注册表；`contrib/codeEditorPart.contribution.ts` 只建立不可缺少的 Code/embedded text runtime 与 typed capability map。简单功能由 browser 主文件就地注册；只有能力注入、两阶段配置或多对象编排才使用独立 `*.contribution.ts`。Editor-owned `BrowserTextModelService` 管理 model reference、dirty/conflict 和保存语义；Workbench 用 `BrowserTextResourceStore` 注入文件 I/O，并拥有保存快捷键、结果呈现和 Pane 生命周期。

### 结构化文档 engine

`DocumentModel` 是 schema 校验的文档树、selection、transaction、history、plugin state 和 serialization 的唯一同步权威。`EditorWidget` 投影结构化节点；Workbench-owned `BrowserDocumentModelService` 负责 reference、working copy 和保存边界。Academic 的 schema、node view、toolbar 和 plugin 由 Workbench `EditorProfile` 组合，formatting/collaboration browser UI 由 `contrib/documentEditor.contribution.ts` 在 editor Academic bundle 中安装。

两个 engine 共享目录和基础设施，不共享 mutation authority。新增所谓“通用”能力时，只有在调用者能够依赖一个小而完整的 capability contract，且两个 engine 的失败和映射语义相同时，才提取共同实现；不得创建带大量可选字段的万能 `IEditorModel`。

## Contribution 装配

Contribution 必须满足以下条件：

- 移除后对应 engine 仍能保持模型有效性和基本编辑正确性；
- 依赖 engine contract，而不是读取产品 ID；
- schema-bearing 能力通过 `EditorProfile` 稳定组合，不能在打开文档后任意开关；
- 不隐式 import 另一个模式 bundle；跨 engine 适配只能 import 所需实现。
- 正式产品只承诺 `editor.all.ts` 及其 Code/Academic 扩展组合，不承诺任意 contribution 子集都能组成受支持的产品。

因此 transaction、selection mapping、IME commit、schema validation 和 model lifecycle 属于 engine；find、folding、suggest、citation toolbar 与 collaboration projection 等属于 contribution 或 profile composition。

## 入口与调用路径

```text
Code build mode ───────┬→ editor.code.all.ts → editor.all.ts ─────────→ standard capability registration
                       └→ workbench/contrib/codeEditor ───→ code/diff pane + input registration
Academic build mode ───┬→ editor.academic.all.ts → editor.all.ts + document contribution
                       └→ workbench/contrib/academic ──────→ profile + document pane registration
                                                               └→ embedded CodeEditorWidget factory

editor.api.ts ─────────────→ TextModel / DocumentModel APIs
editor.main.ts ────────────→ editor.all.ts + editor.api.ts
```

Workbench 模式 contribution 是唯一能力选择点：`editor.all.ts` 固定标准能力，`editor.code.all.ts` 和 `editor.academic.all.ts` 只表达模式差异，并与对应 Workbench contribution 配对。共享入口在窗口启动时只加载一个 bundle；切换模式通过 reload 创建新的 Renderer 生命周期。新增模式必须先登记 `WorkbenchModeId` 并补齐 Browser/Electron 的穷尽 loader 映射；不得在共享 Workbench、widget、model 或 feature controller 内增加模式分支，也不得复制整份标准能力清单。

## 关键实现符号

| 符号 | 责任 | 修改时必须同步检查 |
| --- | --- | --- |
| `TextModel` | 行式文本 mutation、version、history、snapshot | cursor、tracked range、language result version gate、text-engine tests |
| `DocumentModel` | 结构化 transaction、selection mapping、plugin state、history | schema、serialization、collaboration rebase、document-engine tests |
| `CodeEditorWidget` | 行式 DOM projection 与必需 input/navigation surface | viewport、accessibility、embedded adapter、contributed controllers |
| `registerEditorContribution` | 所有 Stanza capability 的进程级静态注册 | `editor.*.all.ts`、text/document 挂载点和 contribution 顺序 |
| `EditorWidget` | 结构化节点、marks、selection 与 node-view lifecycle | schema profile、clipboard、collaboration decoration |
| `EditorProfile` | schema、empty document、node view、toolbar、plugin 和 collaboration schema ID 的稳定组合 | Academic bundle、持久格式兼容性、协作房间兼容性 |
| Workbench `registerEditorPane` | Workbench pane descriptor 注册 | 模式入口、editor ID 唯一性、pane matching 顺序；不得从 `editor` bundle 调用 |

如果 common model 开始 import Workbench/generated DTO、contribution 开始拥有第二套 model state、或产品 ID 出现在 feature/controller 中，即表示所有权已经漂移。

## 失败与兼容边界

- 文本和结构化 model 的同步 mutation 失败必须在提交前抛出，不能留下部分版本、history 或 plugin state。
- 异步语言、diff、文件和协作结果必须按 model version 或服务器版本拒绝过期结果。
- Academic schema 与 `collaborationSchemaId` 是持久兼容边界；改变节点语义时必须同步迁移、serialization 测试和 collaboration 测试。
- Workbench 模式 bundle 在 Renderer 启动时静态装配，不提供运行时卸载 contribution 的承诺。
- Stanza 自有的公开入口、editor ID、content type 和 DOM vocabulary 必须使用 `stanza` 品牌；Workbench 通用 editor part 与主题语义 token 仍由各自 owner 命名。不得重新引入 Alpha/Gama 兼容标识。

## 测试与修改影响

- `test:editor:unit` 编译并运行 editor 内核测试，以及随 owner 迁移到 Workbench 的 code/document pane 与 collaboration adapter 测试。
- `test:editor:browser` 在同一浏览器 suite 内验证 text/document model 挂载点、输入、布局、embedded editor 和可访问性集成。
- `test/architecture/editor-architecture.test.ts` 验证扁平目录、禁止的同步层依赖、两个 engine owner 和模式 bundle。

修改 product composition 时至少运行架构测试和两个 Renderer 类型检查目标；修改 model、input、serialization 或 schema 时运行对应 engine 的 unit/browser suite。浏览器集成测试应在统一 Stanza 测试入口下按具体 model 挂载点命名，不再以历史 engine 代号表达架构所有权。
