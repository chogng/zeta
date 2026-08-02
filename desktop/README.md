# `zeta` Electron Desktop

`zeta` 是 Zeta 的 Electron Desktop 产品线。它由 Renderer、Preload 和 Electron Main 组成，
通过 App Server 使用 Rust 后端；三条公开产品线的关系见
[`docs/product-lines.md`](../docs/product-lines.md)。

`code`、`academic` 与 `complete` 三种内部构建变体的静态入口、输出目录和 contribution 所有权以
[`docs/product-editions.md`](../docs/product-editions.md) 为准；本 README 只记录 Desktop
实现、运行与验证入口。

## 启动项目

首次运行时，先在仓库根目录 `zeta` 下安装依赖：

```bash
corepack pnpm install
```

安装完成后，在仓库根目录执行下面的命令启动桌面端：

```bash
corepack pnpm dev:desktop
```

只开发 Browser Workbench 界面时，在仓库根目录运行：

```bash
corepack pnpm dev:web
```

该命令只同步生成资源并启动监听 `127.0.0.1:5173` 的 Vite 开发服务器，不编译或启动 Rust。
Browser Workbench 使用 disconnected API，因此 UI 可以独立开发；Chat、Explorer、Git、
Terminal 等依赖后端的操作会明确报告不可用。

需要真实 App Server 能力时运行完整 Web 开发模式：

```bash
corepack pnpm dev:web:full
```

完整模式监听 `127.0.0.1:5174`。Browser 通过 Vite 已认证的 HMR WebSocket 连接本地开发
桥接器；桥接器为每个浏览器连接启动独立的 `zeta app-server --listen stdio://` 子进程，
浏览器连接关闭时对应子进程也会被回收。`dev:web:code`、`dev:web:academic`、
`dev:web:complete` 与对应的 `dev:web:full:*` 命令用于显式选择产品版本。

`dev:desktop` 与 `dev:web:full` 会先通过 Node 开发组装器生成
`desktop/.tmp/zeta-package`；其中包含 debug Rust
CLI、锁定版本的 ripgrep 与平台 sandbox helper。开发态和发布态 Electron 都从相同的
`<package>/bin/zeta[.exe]` 入口启动 App Server，区别仅在编译 profile 和 package root。
准备流程只使用 Desktop 已要求的 Node、Rust 和 host archive utility，不安装或调用
Python。`dev:desktop` 随后启动 Vite、主进程、预加载脚本和 Electron；`dev:web:full` 只启动
Vite，并按浏览器连接管理 App Server。启动后不要关闭终端，停止服务可以按 `Ctrl+C`。

不带项目路径启动时，Zeta 使用空窗口上下文。构建完成后，可以通过启动参数打开一个项目目录：

```powershell
corepack pnpm --dir desktop start -- C:\path\to\project
```

也可以显式声明目标类型：

```powershell
corepack pnpm --dir desktop start -- --folder C:\path\to\project
corepack pnpm --dir desktop start -- --workspace C:\path\to\team.zeta-workspace
```

单目录启动时，当前版本会把目录作为 App Server 的受限 workspace root，并通过
`fs/getMetadata`、`fs/readDirectory` 为 Renderer 的 Explorer 提供按需目录枚举，并通过
`fs/readFile` 把不超过 10 MiB 的 UTF-8 文件打开到已注册的文本编辑器。编辑目前只保留在
内存模型中；保存、写入/重命名、文件监听与自动刷新、多根 Workspace 配置解析尚未实现。
Explorer 使用的 Seti manifest 和 WOFF 由
[`zeta-file-icons`](../zeta-rs/file-icons/README.md) 统一拥有，并在 Desktop 构建、测试和
Renderer 类型检查前同步到 `generated/file-icons/`。TypeScript 直接从同步后的 JSON
推导所需结构，不维护额外的 Schema 或生成类型。

代码中 `platform/workspace` 定义当前窗口的 Workspace 模型与上下文，
`platform/workspaces` 负责启动目标解析和后续工作区管理能力。两者不是同一服务的单复数别名。

## Electron 启动门禁

根入口 `src/main.ts` 先同步执行 Electron bootstrap，再加载产品主进程入口；
`code/electron-main/main.ts` 通过 Electron `ready` 事件启动应用，不在 ESM 顶层等待
`app.whenReady()`。`ZetaApplication.startupAfterReady()` 会断言 Electron 已进入 Ready，
从结构上避免入口模块和 `ready` 生命周期互相等待。

Ready 后在后台启动 App Server，并完成 initialize、server identity、protocol version
与 schema hash 校验；门禁通过后才创建业务 Workbench 窗口。主窗口初始保持隐藏，
在 `ready-to-show` 后恢复窗口模式并显示，启动过程不创建额外的 splash 窗口。
门禁失败时使用 Electron 原生对话框提供 Retry/Quit，重试会先把 supervisor 恢复到
stopped 状态。

## Browser Workbench

三个 Browser HTML 入口现在会直接启动对应产品 Workbench。`workbench/browser/web.factory.ts`
拥有自动启动与 `pagehide` 释放，`web.api.ts` 定义 embedder 输入，产品入口仍只选择自身的
Monaco/ProseMirror contribution。

Renderer 控件、Workbench Part 与 CSS 状态的 canonical 所有权规范见
[`docs/ui-styling-ownership.md`](../docs/ui-styling-ownership.md)。
Command、MenuId、Context Key 与菜单型 Toolbar 的 canonical 组合规范见
[`docs/menu-system.md`](../docs/menu-system.md)。

普通 `dev:web`、`dev:renderer` 和静态 Browser 构建未配置 host 时由
`platform/app-server/browser/rendererApi.ts` 提供 disconnected API：UI 正常启动，状态栏显示
App Server 不可用，产品操作明确失败。`dev:web:full` 则由 `web-app-server-vite-plugin.mjs`、
`ViteDevAppServerConnection` 与 `connectViteDevRendererApi()` 组成仅限本机开发的 host，并在
Workbench 启动前注入同一份 `IRendererHost` contract。嵌入方若已实现受认证的远程 transport，必须在产品入口
执行前注入：

```ts
globalThis.zetaWebWorkbenchHost = {
  api: authenticatedRendererApi,
  workspace,
};
```

该对象是进程内 capability，不是可直接从不可信 JSON 反序列化的配置。当前 Rust App Server
仍只支持 `zeta app-server --listen stdio://`。`dev:web:full` 的 WebSocket 只属于 loopback Vite
开发宿主，不是 Rust listener，也不是可部署服务；生产级 HTTP/WebSocket listener、认证、
origin policy 和远程部署尚未实现，因此静态 Browser 构建不能描述为已连接的 Web 客户端。

## Electron sandbox 边界

`base/parts/sandbox/electron-browser/preload.cts` 是主窗口唯一的 preload 入口。它在
`sandbox: true` 与 `contextIsolation: true` 下运行，运行时只能加载 `electron`，并通过
`ISandboxGlobals` 暴露受 `zeta:` 前缀约束的 IPC 与只读进程元数据。

普通 Renderer 中的 `createElectronRendererApi()` 是该底层桥接的唯一产品适配器；Workbench
只在 composition root 消费它生成的 `ZetaElectronRendererApi`，再注册按领域划分的 Workbench Service；
contrib 不直接持有聚合 Renderer Host。Electron Main 的 `registerTrustedIpcRoutes()`
继续负责 sender、main frame、入口 URL 和参数验证。若修改 preload、频道或 API 组装，必须同时
运行 `corepack pnpm --dir desktop build` 与 `corepack pnpm --dir desktop test:main`。跨进程
所有权与安全取舍以 [`docs/zeta-desktop-architecture.md`](../docs/zeta-desktop-architecture.md)
为准。

## 嵌入式浏览器边界

`platform/browser` 提供当前窗口的 `WebContentsView` 平台能力。Workbench 只能通过
`ZetaElectronRendererApi.browserView` 创建、布局、导航、隐藏和关闭目标；Electron Main 中的
`BrowserViewMainService` 持有真实 `WebContentsView`，`browserViewIpcRoutes()` 对每条命令做
exact-shape validation。第三方页面使用独立的临时 partition，默认拒绝权限、下载和 popup，
并且不会加载主窗口的 Zeta preload。

当前只完成平台服务和 renderer API，浏览器编辑器、地址栏、标签页以及 DOM 容器到原生
view 的自动布局绑定尚未实现。跨进程所有权与后续演进以
[`docs/zeta-desktop-architecture.md`](../docs/zeta-desktop-architecture.md) 的 Browser
Capability 章节为准。

## iframe Webview

`platform/webview/browser/webviewElement.ts` 提供 Renderer 内的 `WebviewElement`，用于
Markdown Preview、受控 HTML UI 和后续自定义编辑器。创建者持有并释放该对象，宿主按需挂载
其 `element`；内容通过 `srcdoc` 运行在不含 `allow-same-origin` 的 sandbox 中，并由固定 CSP
禁止网络子资源连接、嵌套 frame 和表单提交。

iframe 内容只能通过一次性获取的 `acquireZetaWebviewApi().postMessage()` 向宿主发送数据；
宿主同时校验消息来源窗口和实例频道。该组件不获得 `ZetaElectronRendererApi`、Electron IPC
或 Node 能力。当前尚无扩展宿主、独立 webview origin、资源 URI 映射和持久化 webview state；
这些不能视为已实现能力。架构边界见
[`docs/zeta-desktop-architecture.md`](../docs/zeta-desktop-architecture.md)。

## Markdown

DOMPurify 作为 `desktop/package.json` 的运行时依赖安装，源码中的唯一直接适配器是
`base/browser/domSanitize.ts`。`base/browser/markdownRenderer.ts` 提供用于 Workbench
标签、Hover 和消息等短内容的 `MarkdownElement`，采用 `marked` 解析后再由 DOMPurify
统一清洗。
`platform/markdown/browser/markdownPreview.ts` 提供完整文档 `MarkdownPreview`，采用
`markdown-it` 解析、同一套 DOMPurify allowlist 清洗，再交给 `WebviewElement` 的
opaque-origin sandbox iframe。解析器输出不能直接写入 DOM 或 iframe。

`workbench/contrib/markdown/browser/markdownDocumentRenderer.ts` 提供产品层
`MarkdownDocumentView`，将平台预览适配为 Editor Part 可持有的视图，并接管链接打开回调。
`markdown.contribution.ts` 由 Workbench contribution 入口静态加载，负责该功能的产品样式，
不重复 platform 的解析与安全实现。

Workbench 静态装配按 host 分层：`workbench.common.main.ts` 加载 Browser 与 Electron
共享的 contribution，`workbench.web.main.ts` 与 `workbench.desktop.main.ts` 只加载各自
host 的 adapter 和 contribution。三个产品入口在 host main 之外继续独立选择 Monaco、
ProseMirror 或两者；新增功能时不得从共享 `Workbench` 构造实现反向导入产品或 host 入口。

当前链接只允许 HTTP、HTTPS 和页内 fragment，并交由宿主处理；图片只允许内嵌的 PNG、
JPEG、GIF 与 WebP。语法高亮、Markdown 插件、Mermaid、KaTeX 和工作区相对资源映射尚未实现。
详细边界见
[`docs/zeta-desktop-architecture.md`](../docs/zeta-desktop-architecture.md#62-markdown)。

## 安装失败时

如果出现 `ERR_PNPM_ENOENT`、`electron_tmp` 或 Electron 目录 rename 错误，请先关闭正在运行的 Electron、Vite 和 Node 进程，然后在仓库根目录重建依赖：

```powershell
Remove-Item -LiteralPath .\node_modules -Recurse -Force
Remove-Item -LiteralPath .\desktop\node_modules -Recurse -Force
corepack pnpm install
corepack pnpm dev:desktop
```

这里只会删除 pnpm 生成的依赖目录，不会删除源码或 `pnpm-lock.yaml`。如果仍然失败，请暂时关闭占用 Electron 文件的杀毒软件实时扫描后重试。

## 常用命令

以下命令均可在仓库根目录执行：

```bash
# 构建桌面端
corepack pnpm build:desktop

# 只运行桌面端主进程测试
corepack pnpm --dir desktop test:main

# 检查 renderer 类型
corepack pnpm --dir desktop typecheck:renderer
```

如果 Electron 的依赖安装被 pnpm 拦截，请确认安装提示中的 `electron` 构建脚本已被允许。

## 第三方许可证

Desktop 直接运行时依赖及其源码内保留的许可证文本见
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。发布打包必须包含该清单及
`desktop/licenses/`，并同时保留 Electron 与所选原生运行时随附的上游 notices。
