# `zeta` Electron Desktop 产品版本、Sessions 与构建边界

> 本文是 Electron Desktop 内部 `code` 与 `academic` 构建变体的 canonical 说明。
> 它们不等同于公开产品线；公开宿主边界见 [`product-lines.md`](product-lines.md)。

## 当前构建变体

| Electron 构建变体 | `ZETA_PRODUCT` | Workbench 入口 | Sessions 入口 | Renderer 输出 |
| --- | --- | --- | --- | --- |
| Zeta Code | `code` | `workbench-code` | `sessions-code` | `desktop/dist/renderer/code` |
| Zeta Academic | `academic` | `workbench-academic` | `sessions-academic` | `desktop/dist/renderer/academic` |

两个产品位于同一仓库、共享 Workbench 基础设施和 Rust App Server，但各自拥有
`applicationId`、`userDataFolderName`、renderer storage namespace 与 App Server profile
root。Electron Main 在持久化服务启动前设置这些路径，因此两者能够同机安装和同时运行。
同一 workspace 仍是共享文件场景，需要文件级协调；它不是产品状态共享。

## Dedicated Sessions

Sessions 是独立页面，不是给 `workbench/browser/layout*` 增加产品分支。普通 Workbench
入口只注册一个 Titlebar action，页面导航到同产品的 sibling Sessions HTML；Electron Main
把 Workbench 与 Sessions HTML 一起加入 trusted IPC allowlist。

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
| Code | 会话列表 / Agent 主区域 / 开发上下文 | 持久 Session/Thread、真实 Chat/Agent 回合、返回普通 Workbench 查看工作区与工具 |
| Academic | 研究会话与文献库 / 阅读-浏览-草稿中心区 / 写作 Agent | PDF、BibTeX、RIS 的本地导入；PDF 阅读；main-owned 原生研究浏览器；草稿到 Agent 的写作请求 |

Academic 的导入文件当前只保留在打开的 Sessions renderer 页中。持久文献库、Zotero 数据库同步、
完整 BibTeX/RIS metadata/citation parser 与文献索引属于后续 Academic domain 服务，不能偷偷
放进 Workbench layout 或 generic session storage。

## 编辑器与默认 Workbench

普通 Workbench 仍使用已有的 immutable `WorkbenchSession` 初始 profile：Code 使用 Aster，
Academic 使用 Aster。它们只决定正常工作台启动时的默认区域和 editor bundle；Dedicated
Sessions 自己构造固定页面，不读取或更改 `WorkbenchLayout`。

产品入口分别静态加载 `editor.code.all.ts` 与 `editor.academic.all.ts`。两个 bundle 都来自同一个扁平的 `src/zeta/editor` 模块；Aster 是统一内核品牌，`TextModel` 与 `DocumentModel` 分别拥有 Text Engine 和 Document Engine 的同步语义。

## 构建与验证

从仓库根目录执行：

```bash
corepack pnpm --dir desktop build:code
corepack pnpm --dir desktop build:academic
```

每次构建必须产生同产品的 Browser 与 Electron Workbench/Sessions HTML。发布包只收录一个
产品 renderer 目录；Main 检测到零个或多个完整产品入口（Workbench 加 Sessions）时会拒绝启动，
避免安装身份依赖用户可控的环境变量。
