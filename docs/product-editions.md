# `zeta` Electron Desktop 产品版本、Sessions 与构建边界

> 本文是 Electron Desktop 内部 `code` 与 `academic` 构建变体的 canonical 说明。
> 它们不等同于公开产品线；公开宿主边界见 [`product-lines.md`](product-lines.md)。

## 快速理解

Code 与 Academic 是同一桌面宿主的两种构建模式，不是两套 Workbench。两种构建共享入口、
布局 profile 和 Workbench runtime；构建模式只静态选择 editor、Tasks、Testing、Debug 等能力
contribution。这个边界沿用 VS Code 的单一 Workbench 与产品级 contribution 装配方式。

| Electron 构建变体 | `ZETA_PRODUCT` | Workbench 入口 | Dedicated Sessions | Renderer 输出 |
| --- | --- | --- | --- | --- |
| Zeta Code | `code` | `workbench` | `sessions-code` | `desktop/dist/renderer/code` |
| Zeta Academic | `academic` | `workbench` | ❌ 未提供 | `desktop/dist/renderer/academic` |

## 当前构建变体

两个构建变体位于同一仓库、共享 Workbench 基础设施和 Rust App Server，但各自拥有
`applicationId`、`userDataFolderName`、renderer storage namespace 与 App Server profile
root。Electron Main 在持久化服务启动前设置这些路径，因此两者能够同机安装和同时运行。
同一 workspace 仍是共享文件场景，需要文件级协调；它不是产品状态共享。

## Dedicated Sessions

Code Sessions 是独立页面，不是给 `workbench/browser/layout*` 增加产品分支。Code 的普通
Workbench 入口只注册一个 Titlebar action，Browser 页面导航到 sibling Sessions HTML；
Electron Main 创建独立 Sessions 窗口，并把对应 HTML 加入 trusted IPC allowlist。

```text
regular Workbench titlebar
        │ Open <Product> Sessions
        ▼
sessions/<product> page
        │ Return to Workbench
        ▼
regular Workbench page
```

`sessions/` 可以依赖 `workbench/` 的可复用 Chat、Markdown 和 renderer capability；
`workbench/` 不得反向导入 `sessions/`。这是避免专用工作台污染默认 Workbench 布局的硬边界。

| Sessions workbench | 固定布局 | 当前能力 |
| --- | --- | --- |
| Code | Titlebar / Sessions Sidebar / 多 Session Grid / Auxiliary Bar | 持久 Session/Thread、完整 ChatPane、草稿延迟持久化、活动叶节点与 Back/Forward、返回普通 Workbench |
| Academic | ❌ 未实现 | Academic 当前只提供普通 Workbench；不得把未来研究工作台描述为现有能力 |

Code Sessions 的 Renderer 实现、状态 owner、执行路径、失败语义和扩展点见
[`desktop/src/zeta/sessions/README.md`](../desktop/src/zeta/sessions/README.md)。Academic 若未来
增加专用研究工作台，必须先新增明确的 product capability 与独立 renderer 入口；PDF、文献库、
Zotero 同步和引用索引等领域能力不得提前放进 Workbench layout 或 generic Session storage。

## 编辑器与默认 Workbench

普通 Workbench 使用唯一的 immutable `defaultWorkbenchSession`：两个构建具有相同初始区域、
默认 view container 和可持久化布局语义。Dedicated Sessions 自己构造固定页面，不读取或更改
`WorkbenchLayout`。

`workbench.ts` 是 Browser 与 Electron 各自唯一的 Workbench renderer 入口；Vite 将
`__ZETA_PRODUCT__` 固定为当前构建模式，再加载 `modes/code` 或 `modes/academic`。模式 contribution
分别静态加载 `editor.code.all.ts` 与 `editor.academic.all.ts`，但不得拥有布局、Part topology 或
第二套 Workbench runtime。两个 editor bundle 都来自同一个扁平的 `src/zeta/editor` 模块；Aster
是统一内核品牌，`TextModel` 与 `DocumentModel` 分别拥有 Text Engine 和 Document Engine 的同步语义。

## 构建与验证

从仓库根目录执行：

```bash
corepack pnpm --dir desktop build:code
corepack pnpm --dir desktop build:academic
```

每次构建必须产生共享命名的 Browser 与 Electron `workbench.html`；只有声明
`dedicatedSessions` 的 Code 还必须产生 Browser 与 Electron Sessions HTML。发布包只收录一个
产品 renderer 目录；Main 会校验该产品声明的完整入口集合，避免安装身份依赖用户可控的环境变量。
