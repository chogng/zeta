# Zeta 产品版本与构建边界

> 本文是 Code、Academic 与 Complete 三个 Desktop 产品版本的规范来源。
> Desktop 进程与安全边界由
> [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) 负责。

## 决策

Zeta 的 Desktop 源码保存在同一仓库，但从三个静态产品入口构建：

| 产品 | `ZETA_PRODUCT` | 入口名 | Renderer 输出 |
| --- | --- | --- | --- |
| Zeta Code | `code` | `workbench-code` | `desktop/dist/renderer/code` |
| Zeta Academic | `academic` | `workbench-academic` | `desktop/dist/renderer/academic` |
| Zeta Complete | `complete` | `workbench-complete` | `desktop/dist/renderer/complete` |

`code` 是未指定 `ZETA_PRODUCT` 时的兼容默认值。三个入口集中在同一个 `src/zeta/code` 源码根；
每次 Vite 构建只把所选产品对应的 Browser/Electron HTML 入口交给 Rollup。

## 所有权与装配

`ProductConfiguration` 拥有稳定产品 ID、展示名称与 `rendererEntry`。Vite、Electron Main
和安装包识别必须消费该映射，不得各自推导入口文件名。共享 `Workbench` 消费产品配置，但
不决定要加载哪些产品功能。

产品入口必须在调用 `startBrowserWorkbench()` 或 `startElectronWorkbench()` 之前导入自己的
contribution：

```text
Code entry ───────→ Code contributions ───────┐
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
- 三个产品使用独立 HTML 标题、`ProductConfiguration` 和 Renderer 输出目录。
- 各 `workbench-*.ts` 直接导入自身需要的编辑器 contribution，不额外拆分产品级
  `workbench-*.contribution.ts` 文件。
- 共享 `EditorPart` 已通过 `EditorInput`、`IEditorPane` 和 `EditorPaneRegistry` 提供真实的编辑器
  宿主边界，包括资源匹配、显式 “Open With”、异步切换取消、布局、可见性、聚焦与释放。
- Code 已从产品入口注册 Monaco pane；它拥有文本模型、语言 worker、布局、焦点和释放。
- Academic 已从产品入口注册 ProseMirror pane；它拥有基础论文 schema、历史、快捷键、布局和释放。
- Main、Preload 和 App Server 当前由三个产品共享，尚未按产品裁剪原生能力或 Rust feature。

因此，Renderer 构建产物和编辑器依赖已经按产品入口分流。Complete 静态组合两套编辑器；
Academic 专有内容类型和 `.zeta-paper`、`.zeta-academic` 默认由 ProseMirror 打开，普通源码及
Markdown 默认由 Monaco 打开。

## 编辑器装配契约

`workbench-code.ts` 只能引入 `editor/monaco/contrib`；
`workbench-academic.ts` 只能引入 `editor/prosemirror/contrib`。共享 Workbench 不得
直接依赖任一编辑器。`workbench-complete.ts` 静态导入这两个 contribution，获得两个编辑器：

```text
Code       → Monaco descriptor
Academic   → ProseMirror descriptor
Complete   → Monaco descriptor + ProseMirror descriptor
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

当前宿主一次只拥有一个活动 pane，不拥有 tab、历史记录、脏状态、保存或恢复语义。这些仍是
后续编辑器/文档服务的职责，不能由 pane 私自发明。

## 扩展点

新增 Code 或 Academic 专属功能时，应分别从 `workbench-code.ts` 或
`workbench-academic.ts` 导入。Complete 入口显式导入两类 contribution，获得这些功能。

Monaco 和 ProseMirror 的模型、插件与 worker 分别归 `editor/monaco` 和
`editor/prosemirror` 子系统所有；产品 contribution 只选择是否装配，不能持有编辑器实现。
共享 `workbench` 同样不得直接依赖具体编辑器。当前 `EditorInput.initialText` 只是文档服务
接入前的内存启动快照，不拥有保存语义。

## 构建与验证

从仓库根目录运行：

```bash
corepack pnpm build:desktop:code
corepack pnpm build:desktop:academic
corepack pnpm build:desktop:complete
```

发布打包必须只收录目标产品的 Renderer 目录。把整个 `dist/renderer` 复制进安装包会破坏产品
隔离；packaged Main 检测到零个或多个完整产品入口时会拒绝启动。

CI 应分别构建三个产品，并验证每个 HTML 入口、产品标题和 Main 选择路径。共享 TypeScript
类型检查不能替代三次入口构建，因为未被选中的静态入口可能存在独立的解析或打包错误。
