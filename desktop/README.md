# Zeta Desktop

Zeta Desktop 是 Zeta 的 Electron 客户端。

Code、Academic 与 Complete 三种构建的静态入口、输出目录和 contribution 所有权以
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

这个命令会先编译 Rust CLI，然后启动 Vite、主进程、预加载脚本和 Electron。启动后不要关闭终端，停止服务可以按 `Ctrl+C`。

不带项目路径启动时，Zeta 使用空窗口上下文。构建完成后，可以通过启动参数打开一个项目目录：

```powershell
corepack pnpm --dir desktop start -- C:\path\to\project
```

也可以显式声明目标类型：

```powershell
corepack pnpm --dir desktop start -- --folder C:\path\to\project
corepack pnpm --dir desktop start -- --workspace C:\path\to\team.zeta-workspace
```

当前版本只建立启动窗口的 Workspace 身份并选择相应窗口策略；目录内容读取、Workspace
配置解析和运行时“打开项目”界面尚未实现。

代码中 `platform/workspace` 定义当前窗口的 Workspace 模型与上下文，
`platform/workspaces` 负责启动目标解析和后续工作区管理能力。两者不是同一服务的单复数别名。

## Electron sandbox 边界

`base/parts/sandbox/electron-browser/preload.cts` 是主窗口唯一的 preload 入口。它在
`sandbox: true` 与 `contextIsolation: true` 下运行，运行时只能加载 `electron`，并通过
`ISandboxGlobals` 暴露受 `zeta:` 前缀约束的 IPC 与只读进程元数据。

普通 Renderer 中的 `createElectronRendererApi()` 是该底层桥接的唯一产品适配器；Workbench
只消费它生成的 `ZetaElectronRendererApi`。Electron Main 的 `registerTrustedIpcRoutes()`
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
