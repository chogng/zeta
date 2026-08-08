# `zeta` Electron Desktop 产品版本与构建边界

> 本文是 `zeta` Electron Desktop 内部 `code`、`academic` 与 `complete` 三个构建变体的规范来源。
> 它不是公开产品线命名的来源；三条产品线见 [`product-lines.md`](product-lines.md)。
> Desktop 进程与安全边界由
> [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) 负责。

## 快速理解

`zeta` 的 Electron Desktop 源码保存在同一仓库，但从三个静态构建入口生成不同变体：

| Electron 构建变体 | `ZETA_PRODUCT` | 入口名 | Renderer 输出 | 公开产品线 |
| --- | --- | --- | --- | --- |
| Default Zeta Desktop | `code` | `workbench-code` | `desktop/dist/renderer/code` | `zeta` |
| Zeta Academic edition | `academic` | `workbench-academic` | `desktop/dist/renderer/academic` | `zeta` |
| Zeta Complete edition | `complete` | `workbench-complete` | `desktop/dist/renderer/complete` | `zeta` |

`code` 是当前未指定 `ZETA_PRODUCT` 时的内部默认构建 ID；它不是 `zeta code` TUI 的产品 ID。
三个入口集中在同一个 `src/zeta/code` 源码根；
每次 Vite 构建只把所选产品对应的 Browser/Electron HTML 入口交给 Rollup。

## 所有权与装配

`ProductConfiguration` 拥有稳定产品 ID、展示名称与 `rendererEntry`。Vite、Electron Main
和安装包识别必须消费该映射，不得各自推导入口文件名。共享 `Workbench` 消费产品配置，但
不决定要加载哪些产品功能。

产品入口必须在调用 `startBrowserWorkbench()` 或 `startElectronWorkbench()` 之前导入自己的
contribution：

```text
Default Zeta entry → Code contributions ──────┐
Academic entry ──→ Academic contributions ────┼→ host main → common main
Complete entry ──→ Code + Academic ───────────┘
```

Browser 入口加载 `workbench.web.main.ts`，Electron Renderer 入口加载
`workbench.desktop.main.ts`；两者再加载 `workbench.common.main.ts`。通用 contribution 只能
从 common main 接入，Browser-only 与 Electron-only adapter 分别由对应 host main 所有。
产品入口不能直接导入另一 host 的 contribution。

共享的 `workbench/browser/workbench.ts` 不得重新导入产品 contribution。出现该依赖意味着产品
装配所有权已经从静态入口漂移回共享外壳，并可能把另一版本的编辑器打进产物。

开发模式下 Electron Main 与 Vite 必须读取相同的 `ZETA_PRODUCT`。Main 使用它选择
`dist/renderer/<product>/electron-browser/workbench/<rendererEntry>.html`，该 URL 同时进入
可信 IPC 入口白名单。安装后的 Main 不信任环境变量，而是要求安装包中只存在一个完整
Renderer 产品目录，并从该目录与入口名确定产品身份。

## 当前实现

- 三个 Browser 入口集中在 `code/browser/workbench/workbench-{code,academic,complete}.*`。
- 三个 Electron Renderer 入口集中在
  `code/electron-browser/workbench/workbench-{code,academic,complete}.*`。
- Renderer host 装配由 `workbench.web.main.ts`、`workbench.desktop.main.ts` 与
  `workbench.common.main.ts` 分层；Electron-only 开发者 action 不进入 Browser 产物。
- 三个 Browser 产品模块加载后会通过 `web.factory.ts` 自动启动 Workbench。Web embedder 可在
  导入产品入口前设置 `globalThis.zetaWebWorkbenchHost`，提供真实 `ZetaRendererApi`、
  workspace 和可选容器。
- 三个 Electron 构建变体使用独立 HTML 标题、`ProductConfiguration` 和 Renderer 输出目录。
- 各 `workbench-*.ts` 直接导入自身需要的 `editor/*/editor.all.ts`，不额外拆分产品级
  `workbench-*.contribution.ts` 文件；`editor.all.ts` 只负责 browser contribution，`editor.api.ts`
  是程序化模型接口，`editor.main.ts` 组合二者，`editor.worker.start.ts` 仅在该 editor 真有 worker runtime 时存在。
- 共享 `EditorPart` 已通过 `EditorInput`、`IEditorPane` 和 `EditorPaneRegistry` 提供真实的编辑器
  宿主边界，包括资源匹配、显式 “Open With”、异步切换取消、布局、可见性、聚焦与释放。
- Default Zeta entry 已从产品入口注册 Alpha pane；Alpha 是普通源码和纯文本资源的默认行编辑器。
- Academic 已从产品入口注册 Gama pane（browser widget 为 `TextEditorWidget`）；它拥有结构化文档 schema、事务、历史、序列化、布局和释放。
- `sessions/browser/*WorkbenchSession.ts` 为 Code、Academic、Complete 分别提供初始 Workbench
  布局 profile；共享 Workbench 只消费通用 profile 契约，并按产品/工作区恢复用户已保存的尺寸和显隐。
- Main、Preload 和 App Server 当前由三个 Electron 构建变体共享，尚未按变体裁剪原生能力或 Rust feature。
- Code、Academic、Complete 使用不同的稳定 `applicationId`、`userDataFolderName` 和 Renderer
  storage namespace；Electron Main 在启动前显式设置 `userData`、`sessionData`、logs 和 crash
  roots，并按产品申请单实例锁。因此 Code 与 Academic 可以同时安装、同时运行，App Server
  仍可复用同一个二进制；各自的 `ZETA_PROFILE_ROOT` 指向各自 user data 下的 `state`。
- 未注入 Web host 的 Browser 页面使用显式 disconnected renderer API：页面和 Workbench
  可以运行，连接状态为 `stopped`，所有 App Server 产品操作以
  `WebAppServerUnavailableError` 失败。当前 Rust App Server 只监听 `stdio://`，尚无浏览器可
  直连的 HTTP/WebSocket transport。

因此，Renderer 构建产物和编辑器依赖已经按产品入口分流。Complete 静态组合 Alpha 与 Gama；
Academic 专有内容类型和 `.zeta-paper`、`.zeta-academic` 默认由 Gama 打开，普通源码及
Markdown 默认由 Alpha 打开。

Workbench session profile 与实时聊天 session 是两层不同的概念：前者只决定产品入口的初始
Workbench 组成，后者由 `IWorkbenchSessionService` 管理当前 thread、transcript 和服务端
lifecycle。`SessionsPart` 是可选的状态投影，不拥有 Workbench 布局拓扑。

## 同机安装与隔离契约

这三个构建变体共享执行内核，但不应共享产品身份或可写数据目录。部署时必须保持以下边界：

| 资源 | Code | Academic | 是否允许共享 |
| --- | --- | --- | --- |
| Workbench / App Server 二进制 | 相同 | 相同 | ✅ 只读代码与协议 |
| installer/application ID | `com.zeta.desktop.code` | `com.zeta.desktop.academic` | ❌ |
| Electron user data | `Zeta` | `Zeta Academic` | ❌ |
| App Server profile root | `<userData>/state` | `<userData>/state` | ❌ |
| 同一 workspace 文件 | 可打开 | 可打开 | 协调访问，不是产品状态共享 |

因此“内核相同”不是冲突源；共用 `userData`、Chromium `sessionData`、App Server 的
`state.sqlite3` 或 installer identity 才会造成配置、登录态、窗口状态、Session/Thread
历史、lease 和单实例锁互相覆盖。当前 Electron 测试传入的显式 `--user-data-dir` 会保留，
方便每个测试使用临时目录；发布包不应靠环境变量选择产品，而应只携带一个 Renderer 产品目录
并使用对应的 installer ID。

## 编辑器装配契约

`workbench-code.ts` 引入 `editor/alpha/editor.all` 与 `codeWorkbenchSession`；
`workbench-academic.ts` 引入 `editor/gama/editor.all` 与 `academicWorkbenchSession`。共享 Workbench 不得
直接依赖任一编辑器。`workbench-complete.ts` 静态导入两个 editor bundle 与 `completeWorkbenchSession`：

```text
Code       → Alpha descriptor
Academic   → Gama descriptor
Complete   → Alpha + Gama descriptors
                         ↓
                 EditorPaneRegistry
                         ↓
                    EditorPart
```

每个编辑器 contribution 只注册一个 `IEditorPaneDescriptor`。`canOpen()` 必须是纯函数，以
`EditorPaneMatch` 声明默认或可选匹配；同分时按注册顺序稳定选择。显式 “Open With” 通过
`preferredEditorId` 选择兼容 pane，候选列表由 `getEditors()` 提供。

`EditorPart` 在新 pane 的 `setInput()` 成功前保留旧 pane。新打开被后续操作替代时会中止
`AbortSignal` 并释放新 pane；实现必须观察该信号。成功切换后，宿主负责隐藏、清空并释放旧
pane。编辑器实现不得自行替换 Editor Part 内容，也不得持有产品全局生命周期之外的 DOM。

当前宿主一次只拥有一个活动 pane，不拥有 tab、历史记录、脏状态、保存或恢复语义。
`ITextFileService` 只拥有 file/bootstrap 内容解析；其余文档生命周期仍是后续 TextFile
model contract 的职责，不能由 pane 私自发明。

## 扩展点

新增 editor browser contribution 时，应从它所属 editor 的 `editor.all.ts` 引入；产品入口只选择
`editor.all.ts` 与对应 Workbench session。Complete 入口显式选择两个 editor bundle，获得两类功能。

Alpha 与 Gama 的模型、插件与 browser runtime 分别归 `editor/alpha`、`editor/gama` 子系统所有；产品 contribution 只选择是否装配，不能持有编辑器实现。
共享 `workbench` 同样不得直接依赖具体编辑器。
`EditorInput.initialText` 是 `ITextFileService` 优先使用的内存启动快照，不拥有保存语义。

## 构建与验证

从仓库根目录运行：

```bash
corepack pnpm build:desktop:code
corepack pnpm build:desktop:academic
corepack pnpm build:desktop:complete
```

发布打包必须只收录目标产品的 Renderer 目录。把整个 `dist/renderer` 复制进安装包会破坏产品
隔离；packaged Main 检测到零个或多个完整产品入口时会拒绝启动。

CI 应分别构建三个 Electron 构建变体，并验证每个 HTML 入口、产品标题和 Main 选择路径。共享 TypeScript
类型检查不能替代三次入口构建，因为未被选中的静态入口可能存在独立的解析或打包错误。
