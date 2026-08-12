# Editor

> 本文拥有 Renderer editor 模块的实现和修改契约。跨 Workbench、文件、语言服务与 Rust App Server 的系统边界见 [`docs/editor-architecture.md`](../../../../docs/editor-architecture.md)。文本内核和结构化文档内核的详细行为分别见 [`text-engine.md`](./text-engine.md) 与 [`document-engine.md`](./document-engine.md)。

## 快速理解

Editor 采用与 VS Code `src/vs/editor` 一致的扁平职责分区：`common` 保存 DOM-free 模型与算法，`browser` 保存 DOM 投影和宿主适配，`contrib` 保存可装配能力，`test` 保存内核级回归测试。Alpha 和 Gama 不再形成两套目录或产品入口；它们分别由 `TextModel` 和 `DocumentModel` 表达两个独立的 canonical engine。

| 产品或调用方式 | 加载入口 | 得到的能力 |
| --- | --- | --- |
| Code | `editor.code.all.ts` | 行式代码编辑器、diff pane 和代码编辑能力 |
| Academic | `editor.academic.all.ts` | 结构化文档 profile、学术 schema、citation、collaboration，以及按需嵌入的行式文本 widget |
| 完整宿主和集成测试 | `editor.all.ts` | Code 与 Academic 两组 contribution |
| DOM-free 程序化调用 | `editor.api.ts` | `TextModel`、`DocumentModel`、schema、transaction 和坐标值对象；不注册 pane |
| 完整程序化入口 | `editor.main.ts` | `editor.all.ts` 与 `editor.api.ts` 的组合 |

## 所有权与依赖方向

| 目录 | 允许依赖 | 拥有 | 不得拥有 |
| --- | --- | --- | --- |
| `common/core` | `base/common` | 文本坐标、文档坐标、selection、纯变换算法 | DOM、Workbench service、App Server DTO |
| `common/model` | `common/core`、`base/common` | `TextModel`、`DocumentModel`、history、schema、transaction、serialization | 文件传输、浏览器 focus、产品 profile |
| `common/cursor`、`common/viewModel`、`common/viewLayout` | 文本内核与 `base/common` | 行式编辑器实例状态和纯布局投影 | DOM 和产品判断 |
| `browser` | `common`、`base/browser` 和显式前端 service contract | code/document/diff widget、输入、viewport、pane 与 service adapter | 产品版本选择 |
| `contrib/<feature>` | 对应 engine 的最小 contract | 可移除的编辑能力及其命令、状态和投影 | 第二套 model、产品级 `if code/academic` |
| `editor.*.all.ts` | contribution entry | 静态产品装配 | 模型或功能实现 |

依赖必须保持 `contrib/browser → browser/common → common → base` 的方向。`src/zeta/base` 不得反向引用 editor。结构化文档可以通过 Workbench-owned `IEmbeddedTextEditor` contract 使用 `CodeEditorWidget`，但 `DocumentModel` 不得依赖 `TextModel`，`TextModel` 也不得依赖 document schema。

## 两个 engine

### 行式文本 engine

`TextModel` 是纯文本、版本、transaction、undo/redo、tracked range 和 snapshot 的唯一同步权威。`CodeEditorWidget` 与 `EditorPart` 投影它，但不拥有共享 model。`BrowserTextModelService` 将 Workbench 文件生命周期适配为 editor-owned model reference。

### 结构化文档 engine

`DocumentModel` 是 schema 校验的文档树、selection、transaction、history、plugin state 和 serialization 的唯一同步权威。`EditorWidget` 投影结构化节点；`BrowserDocumentModelService` 负责 reference、working copy 和保存边界。Academic 的 schema、node view、toolbar 和 plugin 由 `EditorProfile` 组合。

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
Code product entry ───────→ editor.code.all.ts ───────→ code/diff pane registration
Academic product entry ───→ editor.academic.all.ts ───→ academic profile registration
                                                    └─→ embedded CodeEditorWidget adapter

editor.api.ts ─────────────→ TextModel / DocumentModel APIs
editor.main.ts ────────────→ editor.all.ts + editor.api.ts
```

`editor.code.all.ts` 和 `editor.academic.all.ts` 是唯一产品选择点。新增版本应新增一个静态 bundle，组合已有 contribution；不得在 widget、model 或 feature controller 内增加产品分支。

## 关键实现符号

| 符号 | 责任 | 修改时必须同步检查 |
| --- | --- | --- |
| `TextModel` | 行式文本 mutation、version、history、snapshot | cursor、tracked range、language result version gate、text-engine tests |
| `DocumentModel` | 结构化 transaction、selection mapping、plugin state、history | schema、serialization、collaboration rebase、document-engine tests |
| `CodeEditorWidget` | 行式 DOM projection 与 controller composition | input、viewport、accessibility、embedded adapter |
| `EditorWidget` | 结构化节点、marks、selection 与 node-view lifecycle | schema profile、clipboard、collaboration decoration |
| `EditorProfile` | schema、empty document、node view、toolbar、plugin 和 collaboration schema ID 的稳定组合 | Academic bundle、持久格式兼容性、协作房间兼容性 |
| `registerEditorPane` | Workbench pane descriptor 注册 | bundle import、editor ID 唯一性、pane matching 顺序 |

如果 common model 开始 import Workbench/generated DTO、contribution 开始拥有第二套 model state、或产品 ID 出现在 feature/controller 中，即表示所有权已经漂移。

## 失败与兼容边界

- 文本和结构化 model 的同步 mutation 失败必须在提交前抛出，不能留下部分版本、history 或 plugin state。
- 异步语言、diff、文件和协作结果必须按 model version 或服务器版本拒绝过期结果。
- Academic schema 与 `collaborationSchemaId` 是持久兼容边界；改变节点语义时必须同步迁移、serialization 测试和 collaboration 测试。
- Product bundle 是构建时静态选择，不提供运行时卸载 contribution 的承诺。
- 现有 `Alpha*`、`Gama*` editor ID 和 CSS class 可作为兼容 vocabulary 保留；目录所有权和新公共抽象不得继续使用它们区分产品。

## 测试与修改影响

- `test:editor:unit` 编译并运行 `src/zeta/editor/**/test/**/*.test.ts`。
- `test:alpha:browser` 仍验证行式 engine 的浏览器输入、布局和可访问性集成。
- `test:gama:browser` 仍验证结构化 engine 与 embedded text editor 的浏览器集成。
- `test/architecture/editor-architecture.test.ts` 验证扁平目录、禁止的同步层依赖、两个 engine owner 和产品 bundle。

修改 product composition 时至少运行架构测试和两个 Renderer 类型检查目标；修改 model、input、serialization 或 schema 时运行对应 engine 的 unit/browser suite。浏览器集成测试名称暂时保留历史命令兼容性，不代表源代码仍由 `alpha/` 或 `gama/` 目录拥有。
