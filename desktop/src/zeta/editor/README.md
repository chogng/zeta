# Aster

> Aster 是 Zeta 的可组装编辑器内核，类似 Monaco 在 VS Code 架构中的位置；`editor/` 只是它的扁平领域目录，不是第二个品牌或额外架构层。跨 Workbench、文件、语言服务与 Rust App Server 的系统边界见 [`docs/editor-architecture.md`](../../../../docs/editor-architecture.md)。Text Engine 和 Document Engine 的详细行为分别见 [`text-engine.md`](./text-engine.md) 与 [`document-engine.md`](./document-engine.md)。

## 快速理解

Aster 采用与 VS Code `src/vs/editor` 一致的扁平职责分区：`common` 保存 DOM-free 模型与算法，`browser` 保存 DOM 投影和宿主适配，`contrib` 保存可装配能力，`test` 保存内核级回归测试。Aster 只有一个源码域和一套公开入口；真正不同的底层对象分别由 `TextModel` 和 `DocumentModel` 表达。

| 产品或调用方式 | 加载入口 | 得到的能力 |
| --- | --- | --- |
| Code editor 能力 | `editor.code.all.ts` | 行式编辑内核与 Code 所选 editor contribution；不注册 Workbench pane |
| Academic editor 能力 | `editor.academic.all.ts` | 结构化文档 schema、citation、collaboration 与嵌入式行编辑能力；不注册 Workbench pane |
| 完整 editor 能力集 | `editor.all.ts` | Code 与 Academic 两组 editor contribution |
| DOM-free 程序化调用 | `editor.api.ts` | `TextModel`、`DocumentModel`、schema、transaction 和坐标值对象；不注册 pane |
| 完整程序化入口 | `editor.main.ts` | `editor.all.ts` 与 `editor.api.ts` 的组合 |

## 所有权与依赖方向

| 目录 | 允许依赖 | 拥有 | 不得拥有 |
| --- | --- | --- | --- |
| `common/core` | `base/common` | 文本坐标、文档坐标、selection、纯变换算法 | DOM、Workbench service、App Server DTO |
| `common/model` | `common/core`、`base/common` | `TextModel`、`DocumentModel`、history、schema、transaction、serialization | 文件传输、浏览器 focus、产品 profile |
| `common/cursor`、`common/viewModel`、`common/viewLayout` | 文本内核与 `base/common` | 行式编辑器实例状态和纯布局投影 | DOM 和产品判断 |
| `browser` | `common`、`base/browser` 和显式前端 service contract | code/document/diff widget、输入、viewport、contribution registry 与 editor-facing runtime adapter | Workbench pane/input、文件/working-copy 生命周期、产品版本选择 |
| `contrib/<feature>` | 对应 engine 的最小 contract | 可移除的编辑能力及其命令、状态和投影 | 第二套 model、产品级 `if code/academic` |
| `editor.*.all.ts` | contribution entry | 静态 editor 能力装配 | Workbench pane/input 注册、模型或功能实现 |
| `workbench/contrib/{codeEditor,documentEditor,academic}` | Editor 与 Workbench contract | pane/input、产品 profile、embedded factory 和服务接线 | 编辑事务、selection、viewport 或 feature controller |

依赖必须保持 `workbench → editor/contrib → editor/browser/common → editor/common → base` 的方向。`src/zeta/editor` 的生产代码不得反向引用 Workbench，`src/zeta/base` 也不得反向引用 editor。结构化文档通过 editor-owned `IEmbeddedTextEditorFactory` contract 请求行编辑能力，由 Workbench 提供具体 factory；`DocumentModel` 不得依赖 `TextModel`，`TextModel` 也不得依赖 document schema。

## 两个 engine

### 行式文本 engine

`TextModel` 是纯文本、版本、transaction、undo/redo、tracked range 和 snapshot 的唯一同步权威。`CodeEditorWidget` 与 `EditorPart` 投影它，但不拥有共享 model。`browser/editorContribution.ts` 保存 bundle 静态注册的 feature group；`contrib/codeEditorPart.contribution.ts` 提供不可缺少的 Code/embedded text runtime，`find`、`quickAccess`、`smartSelect`、`fontZoom` 等可选控制器由各自 `*.contribution.ts` 注册，并由 bundle 显式选择。Editor-owned `BrowserTextModelService` 管理 model reference、dirty/conflict 和保存语义；Workbench 只用 `BrowserTextResourceStore` 注入文件 I/O。

### 结构化文档 engine

`DocumentModel` 是 schema 校验的文档树、selection、transaction、history、plugin state 和 serialization 的唯一同步权威。`EditorWidget` 投影结构化节点；Workbench-owned `BrowserDocumentModelService` 负责 reference、working copy 和保存边界。Academic 的 schema、node view、toolbar 和 plugin 由 Workbench `EditorProfile` 组合，formatting/collaboration browser UI 由 `contrib/documentEditor.contribution.ts` 在 editor Academic bundle 中安装。

两个 engine 共享目录和基础设施，不共享 mutation authority。新增所谓“通用”能力时，只有在调用者能够依赖一个小而完整的 capability contract，且两个 engine 的失败和映射语义相同时，才提取共同实现；不得创建带大量可选字段的万能 `IEditorModel`。

## Contribution 装配

Contribution 必须满足以下条件：

- 移除后对应 engine 仍能保持模型有效性和基本编辑正确性；
- 依赖 engine contract，而不是读取产品 ID；
- schema-bearing 能力通过 `EditorProfile` 稳定组合，不能在打开文档后任意开关；
- 不隐式 import 另一个产品 bundle；跨 engine 适配只能 import 所需实现。

因此 transaction、selection mapping、IME commit、schema validation 和 model lifecycle 属于 engine；find、folding、suggest、citation toolbar 与 collaboration projection 等属于 contribution 或 profile composition。

## 入口与调用路径

```text
Code product entry ───────┬→ editor.code.all.ts ─────────────→ editor capability registration
                          └→ workbench/contrib/codeEditor ───→ code/diff pane + input registration
Academic product entry ───┬→ editor.academic.all.ts ─────────→ editor capability registration
                          └→ workbench/contrib/academic ──────→ profile + document pane registration
                                                               └→ embedded CodeEditorWidget factory

editor.api.ts ─────────────→ TextModel / DocumentModel APIs
editor.main.ts ────────────→ editor.all.ts + editor.api.ts
```

产品入口是唯一产品选择点：它同时选择一个 `editor.*.all.ts` 能力 bundle 和对应的 Workbench contribution。新增版本应在产品入口组合已有能力与宿主适配；不得在 widget、model 或 feature controller 内增加产品分支。

## 关键实现符号

| 符号 | 责任 | 修改时必须同步检查 |
| --- | --- | --- |
| `TextModel` | 行式文本 mutation、version、history、snapshot | cursor、tracked range、language result version gate、text-engine tests |
| `DocumentModel` | 结构化 transaction、selection mapping、plugin state、history | schema、serialization、collaboration rebase、document-engine tests |
| `CodeEditorWidget` | 行式 DOM projection 与必需 input/navigation surface | viewport、accessibility、embedded adapter、contributed controllers |
| `registerEditorContribution` | 所有 Aster capability 的进程级静态注册 | `editor.*.all.ts`、text/document 挂载点和 contribution 顺序 |
| `EditorWidget` | 结构化节点、marks、selection 与 node-view lifecycle | schema profile、clipboard、collaboration decoration |
| `EditorProfile` | schema、empty document、node view、toolbar、plugin 和 collaboration schema ID 的稳定组合 | Academic bundle、持久格式兼容性、协作房间兼容性 |
| Workbench `registerEditorPane` | Workbench pane descriptor 注册 | 产品入口、editor ID 唯一性、pane matching 顺序；不得从 `editor` bundle 调用 |

如果 common model 开始 import Workbench/generated DTO、contribution 开始拥有第二套 model state、或产品 ID 出现在 feature/controller 中，即表示所有权已经漂移。

## 失败与兼容边界

- 文本和结构化 model 的同步 mutation 失败必须在提交前抛出，不能留下部分版本、history 或 plugin state。
- 异步语言、diff、文件和协作结果必须按 model version 或服务器版本拒绝过期结果。
- Academic schema 与 `collaborationSchemaId` 是持久兼容边界；改变节点语义时必须同步迁移、serialization 测试和 collaboration 测试。
- Product bundle 是构建时静态选择，不提供运行时卸载 contribution 的承诺。
- Aster 自有的公开入口、editor ID、content type 和 DOM vocabulary 必须使用 `aster` 品牌；Workbench 通用 editor part 与主题语义 token 仍由各自 owner 命名。不得重新引入 Alpha/Gama 兼容标识。

## 测试与修改影响

- `test:editor:unit` 编译并运行 editor 内核测试，以及随 owner 迁移到 Workbench 的 code/document pane 与 collaboration adapter 测试。
- `test:editor:browser` 在同一浏览器 suite 内验证 text/document model 挂载点、输入、布局、embedded editor 和可访问性集成。
- `test/architecture/editor-architecture.test.ts` 验证扁平目录、禁止的同步层依赖、两个 engine owner 和产品 bundle。

修改 product composition 时至少运行架构测试和两个 Renderer 类型检查目标；修改 model、input、serialization 或 schema 时运行对应 engine 的 unit/browser suite。浏览器集成测试应在统一 Aster 测试入口下按具体 model 挂载点命名，不再以历史 engine 代号表达架构所有权。
