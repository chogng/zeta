# Zeta Desktop 架构与协作边界

> 负责人：Desktop 开发者
> Rust 对接负责人：zeta-rs 开发者
> 当前开发基线：[`zeta-app-server-api.md`](zeta-app-server-api.md)
> 产品装配与构建版本：[`product-editions.md`](product-editions.md)

## 1. 目标

Desktop 是 Zeta 的 Electron 富客户端，负责窗口、浏览器、系统能力和 UI，不拥有
Session、Thread、Turn、ThreadItem、审批策略或持久化状态机。

Desktop 只能通过版本化 App Server API 使用 zeta-rs：

```text
Renderer
  → typed Preload API
  → Electron Main
  → JSON-RPC / JSONL / stdio
  → zeta app-server
```

Desktop 禁止执行 `zeta ask ...` 后解析终端输出，也禁止直接链接 `zeta-core`。

## 2. Desktop 所有权

Desktop 负责：

- Electron Main、Preload、Renderer；
- App Server 进程启动、初始化、监督、重启和关闭；
- 窗口、菜单、快捷键、命令面板；
- Browser View、Tab、BrowserSession、CDP 和下载；
- Renderer 纯 UI 状态与服务端状态投影；
- 宿主权限、导航策略、origin 策略；
- Desktop 端集成测试。

Desktop 不负责：

- Session、Thread、Turn、ThreadItem、Tool Call 的权威状态；
- Agent 规划和工具循环；
- 是否需要审批的业务策略；
- rollout、SQLite 投影和 Thread writer lease；
- 模型供应商与长期凭据持久化；
- Rust 协议 DTO 的定义。

### 2.1 新功能归属判断

不能因为功能从 UI 进入，就把整项功能都归给 Renderer。设计新功能时，按下面的顺序判断并
拆分职责：

1. 没有 Desktop 时，CLI、TUI 或远程客户端是否仍需要相同语义？如果需要，权威行为和共享
   contract 属于 Rust，并通过 App Server 暴露。
2. 功能是否修改权威状态、访问磁盘或网络、执行进程，或者承担权限与安全校验？如果是，它
   不能只在 Renderer 实现。跨客户端的产品语义归 Rust；Desktop 独有的宿主能力归 Electron
   Main，并通过窄的 typed Preload API 暴露。
3. 功能是否只决定如何显示、如何交互，或维护可丢弃且可重建的视图状态？如果是，它属于
   Renderer。
4. 如果以上答案跨越多层，就把它实现为纵向功能，不把后端语义复制到前端，也不把 UI 状态
   塞进 Rust。

前端可以为了即时反馈重复一部分格式校验，但这不替代权威 owner 在可信边界内重新校验。
Renderer 不能因为已经校验过输入，就获得直接使用 `fs`、网络或任意 IPC/RPC 的权限。

以未来增加 `Files` 能力为例，下表说明职责放置规则，不表示这些 API 当前已经实现：

| 能力 | Owner |
| --- | --- |
| 文件树渲染、选中、展开、快捷键、加载态 | Renderer |
| 系统文件选择器、在原生文件管理器中显示 | Electron Main / Preload |
| 跨客户端的目录枚举、读写、重命名、搜索和 workspace 边界校验 | Rust / App Server |
| 文件位置 identity | 共享 URI contract；Renderer 只维护其视图投影 |
| 跨重启的领域 `FileId` 或 `DocumentId` | 拥有该生命周期的 Rust 领域模型 |
| Tab、Pane 等纯 UI 实例 ID | Renderer |

因此，一项完整功能可以具有一条跨层执行路径：

```text
Renderer component
  → UI command
  → typed Preload API
  → Electron Main
  → typed App Server method
  → Rust authority
```

## 3. 目录边界

```text
desktop/
├── src/
│   ├── main.ts
│   ├── bootstrap.ts
│   └── zeta/
│       ├── base/
│       ├── code/
│       ├── editor/
│       ├── platform/
│       ├── product/
│       └── workbench/
├── generated/
│   └── app-server/
├── package.json
├── tsconfig.main.json
├── tsconfig.preload.json
└── tsconfig.renderer.json
```

`src/` 根目录属于宿主进程启动侧。`bootstrap.ts` 只配置必须在 Electron `ready`
之前生效的进程级策略，`main.ts` 在 bootstrap 完成后加载 Zeta 应用入口。
`src/zeta/` 是产品源码命名空间；其中 `code/electron-main/main.ts` 选择产品并创建
`ZetaApplication`，`code/electron-main/app.ts` 持有服务、窗口、IPC 与退出生命周期。
产品功能不得反向进入根 bootstrap。

产品主进程入口同步注册 Electron `ready` 监听器；异步启动链只能从该监听器触发，
不得在 ESM 顶层等待一个内部再调用 `app.whenReady()` 的 Promise。
`ZetaApplication.startupAfterReady()` 断言 Ready 前置条件，并先创建无 preload、无脚本、
无业务 IPC 的 `StartupWindow`。该窗口属于启动恢复界面，不是业务 UI；App Server gate
成功后才创建 Workbench。gate 失败时，原生 Retry/Quit 对话框允许 supervisor 回到
stopped 后重新初始化，或按正常退出生命周期关闭应用。

`desktop/generated/` 由 zeta-rs 协议生成命令更新，不手写 wire DTO。
生成的 `APP_SERVER_SCHEMA_HASH` 是 bundled Desktop 的 exact-schema 基线；Electron Main
必须比较 initialize response，hash 不一致时不得创建业务窗口或进入 Ready。

## 4. Main Process

Main 必须：

1. 从应用包内确定的绝对路径启动 `zeta app-server --listen stdio://`；
2. 使用 `shell: false`，只传递环境变量 allowlist；
3. 在创建业务 UI 前完成 `initialize`；
4. 校验 protocol version、schema hash 和 server build；
5. 将 stdout 仅交给 JSONL 协议解析器；
6. 对 stderr 做大小限制和 secret 脱敏；
7. 为启动、初始化、请求和关闭设置 deadline；
8. 采用有上限的指数退避处理崩溃重启；
9. 校验每个 Renderer IPC 的 sender、frame URL、origin 和参数；
10. 持有 Browser Target 与 Resource 的宿主侧所有权。

Main 不把 `ipcRenderer`、`fs`、`child_process`、`webContents` 或任意 JSON-RPC method
直接暴露给 Renderer。

当前 `ChildProcessJsonlTransport` 将子进程 stream lifecycle 与 JSON-RPC pairing 分开。它在
积累无限 buffer 前按原始 byte 拒绝超过 1 MiB 的 frame，只接受严格 LF 和有效 UTF-8；
outbound write 同时等待 callback 与 drain，并限制 pending write 数。child/stdio 任一错误
都会关闭 transport；stderr 只保留 64 KiB ring，诊断读取时脱敏 credential。`close()` 异步、
幂等，并在 graceful deadline 后强制终止。`npm run test:main` 覆盖分片 UTF-8、超限 frame、
非法 framing、backpressure、stderr 和 close。

`JsonRpcPeer` 在 transport 之上负责双向 JSON-RPC envelope、request ID pairing、remote
error、timeout/abort、late/unknown/duplicate response、入站 handler cancellation、pending
上限和 listener 隔离。协议生成器输出 `APP_SERVER_METHODS` 与
`APP_SERVER_NOTIFICATIONS` typed definitions，Electron Main 通过 `AppServerClient` 使用；
产品代码不能传任意 method string 或手写 result 泛型。

`AppServerSession` 独占一个 peer，只有 initialize response 同时通过 server identity、
protocol version 和 schema hash gate 后才进入 Ready，并保存协商后的 server
info/capabilities。

`AppServerSession` 是 connection lifecycle。它不是 canonical 产品 `Session`，不得保存
产品 Session membership、lineage 或权威业务状态；Renderer 只维护可丢弃并可 resync 的
Session/Thread projection。
`AppServerSupervisor` 只接受绝对 executable、显式 child environment allowlist，并管理
Stopped/Starting/Initializing/Ready/Stopping/Crashed/Restarting 状态、initialize deadline、
有界指数退避和 crash budget。崩溃会拒绝旧 Session 的 pending request；新 Session 不自动
重放结果未知的副作用操作。

结构化 IPC router 集中注册有限 channel，并在调用 validator/handler 前同时验证目标
webContents、main frame identity 和确切入口 URL。当前窄 IPC surface 对 params 做
exact-shape validation；unknown field、错误 enum、空 ID 或畸形 Turn input 均不会到达
App Server。协议生成 runtime validator 后，应替换这些同形显式 validator 的来源而不改变
router 边界。

### 4.1 Workspace 身份与窗口策略

当前实现明确区分两个所有权边界：

- `platform/workspace`（单数）定义一个窗口当前工作区的模型、结构化标识、
  `WorkbenchState` 和 `IWorkspaceContextService`；
- `platform/workspaces`（复数）负责解析、识别和管理工作区。当前已实现启动目标解析，
  最近项目、运行时切换和 Untitled Workspace 尚未实现。

Desktop 在创建窗口前由 `WorkspacesMainService.resolveStartupWorkspace()` 解析一次启动参数，
并产生不可变的 `IAnyWorkspaceIdentifier`：

- 无项目参数为 `Empty`；
- 目录参数或 `--folder <path>` 为 `Folder`；
- `.zeta-workspace` 文件或 `--workspace <path>` 为 `Workspace`。

`resolveWorkspaceOpenTarget()` 只在 Node/Electron Main 中规范化路径、判断文件类型并为
Folder/Workspace 产生稳定 ID。标识采用 `{ id }`、`{ id, uri }` 或
`{ id, configPath }` 的结构，不存储重复的 `WorkbenchState` 判别字段。窗口状态策略从标识
推导状态：`EMPTY` 映射到 `1200 × 800` 默认窗口，`FOLDER` 和 `WORKSPACE` 映射到
`1440 × 900` 默认窗口。`WindowsStateHandler` 在单个 `windowsState` 记录中持有
`lastActiveWindow` 和 `openedWindows`；每个窗口使用 `workspaceIdentifier`、`folder` 或
`backupPath` 绑定其 UI state。恢复时先匹配具体 Workspace/Folder/空窗口备份，再回退到
last-active state，最后才使用默认尺寸。旧的 `windowState` 与 `windowState.empty` 键不会迁移
或读取。

Renderer 通过受信 IPC route 和 `workspace.getWorkspace()` 读取该身份，并在
`parseWorkspaceIdentifier()` 校验和恢复 URI。`WorkspaceContextService` 根据该标识构造当前
`IWorkspace`，并从 `configuration` 或单根 `folders` 推导 `WorkbenchState`。Workbench
contribution 不得通过该服务直接访问文件系统；跨客户端目录读写、搜索和 workspace 边界
授权仍属于 Rust / App Server。

当前限制：

- Workspace 身份只在启动时确定，尚无运行时打开、关闭或切换项目流程；
- `.zeta-workspace` 当前只作为窗口身份，尚未定义或解析其内容；
- 普通单文件参数仍属于空窗口，文件编辑器尚未实现；
- 当前 `WorkspacesMainService` 只负责启动目标解析，最近项目、多窗口创建和 workspace
  配置管理尚未实现；`windowsState` 已保留多窗口恢复数据形状，但当前只写入单个主窗口；
- 空窗口 backup service 尚未实现，因此当前启动路径没有可传给 `WindowsStateHandler` 的
  `backupPath`，无备份的空窗口只能使用 last-active fallback；
- 启动目标无效时记录错误并安全回退到空窗口。

## 5. Sandbox Bridge 与 Renderer API

Electron sandbox 边界分为两层。`ISandboxGlobals` 是 preload 唯一暴露到主世界的底层桥接：
它只包含只读进程元数据，以及受 `zeta:` 频道前缀约束的 `invoke` / `on`。preload 必须保持
自包含，运行时除 `electron` 外不得加载任何模块，也不得把 Electron event 对象传给 Renderer。
构建后的 preload 由 `verify-sandbox-preload.mjs` 检查这一约束。

`createElectronRendererApi()` 是该桥接的唯一产品适配器。它在普通 Renderer bundle 中引用频道
常量，并组装领域化、强类型、可枚举的 `ZetaElectronRendererApi`。领域方法由其父接口
`ZetaRendererApi` 定义，Electron 专属能力保持以下精确形状：

```ts
interface ZetaElectronRendererApi extends ZetaRendererApi {
  readonly environment: IRuntimeEnvironment;
  readonly browserView: IBrowserViewApi;
  readonly configuration: IConfigurationApi;
  readonly keybindings: IKeybindingsResourceApi;
  readonly nativeContextMenu: INativeContextMenuApi;
  readonly nativeMenubar: INativeMenubarApi;
  readonly workspace: IWorkspaceContextApi;
}
```

Workbench 和其他产品代码禁止直接导入 sandbox globals，也禁止提供绕过
`ZetaElectronRendererApi` 的通用 App Server 调用：

```ts
execute(method: string, params?: unknown): Promise<unknown>
```

## 6. Renderer

Renderer 负责 Command Registry、路由、组件、输入框、虚拟列表和状态投影。

```text
button / menu / shortcut
  → UI Command
  → typed renderer API
  → sandbox IPC bridge
  → trusted IPC route
  → domain RPC
```

Renderer 不复制 Rust 状态机。遇到 durable `sequence` 或 `streamCursor` 空洞时，停止合并
当前实体，并通过 `session/subscribe` 或 `thread/subscribe` 获取权威 snapshot + gap。

### 6.1 Editor 宿主

`EditorPart` 是 Workbench 中央编辑区域的唯一宿主。`EditorInput` 表示待打开资源；
`IEditorPane` 定义编辑器真正共享的创建、输入、取消、布局、可见性、聚焦与释放语义；
`EditorPaneRegistry` 负责默认匹配、候选枚举和显式编辑器选择。具体产品装配规则由
[`product-editions.md`](product-editions.md) 负责。

打开新输入时，旧 pane 保持可见，直到新 pane 的异步 `setInput()` 成功。失败不会破坏当前
编辑器；被后续打开或普通内容替代时，宿主中止 `AbortSignal` 并释放候选 pane。成功切换后由
宿主隐藏、清空并释放旧 pane。当前只实现单活动 pane，尚无 tab、文档模型、脏状态、保存、
备份或恢复协议。

### 6.2 iframe Webview

当前 `WebviewElement` 是 Renderer 内用于受控 HTML 的可释放组件，并暴露可由宿主挂载的
iframe 元素。它适合 Markdown Preview、产品内 HTML 面板和后续自定义编辑器，不负责完整
网页浏览、导航历史、Cookie、CDP 或 Agent Browser Target；后者属于第 7 节的
`WebContentsView` 能力。

`WebviewElement` 创建 `srcdoc` iframe，并固定以下边界：

```text
sandbox: allow-scripts
无 allow-same-origin / forms / popups / downloads / top-navigation
opaque origin + credentialless
固定 iframe CSP 与 document CSP
无 connect / nested frame / object / form action
无 Electron preload、Zeta renderer API 或 Node capability
```

内容通过 `acquireZetaWebviewApi().postMessage()` 发送 structured-clone 数据。宿主只接收
`event.source === iframe.contentWindow` 且实例 channel 匹配的 envelope；宿主向 iframe
发送消息时因为 opaque origin 必须使用 `targetOrigin: "*"`，iframe 内容因此有义务检查
`event.source === parent`。

当前实现只拥有 DOM sandbox、HTML replacement、focus、双向 message 与 deterministic
disposal。扩展宿主、独立 origin endpoint、远程/本地资源映射、端口映射、find widget、
state persistence 和权限扩展均尚未实现。引入这些能力时必须保留独立 origin，不能通过加入
`allow-same-origin` 来绕过资源加载问题。当前也尚未接管 iframe 自身的页面跳转；在加入链接
打开策略前，调用方只应提供产品控制的 HTML。

### 6.3 Markdown

当前 Renderer 有两条 Markdown 渲染路径，但共享同一个最终安全边界：

```text
Workbench 短内容
  → marked
  → DOMPurify allowlist
  → MarkdownElement（普通 DOM）

完整文档预览
  → markdown-it
  → DOMPurify allowlist
  → MarkdownPreview
  → WebviewElement（opaque-origin sandbox iframe）
```

`base/browser/domSanitize.ts` 是 DOMPurify 的唯一直接适配器，为目标 document 创建隔离的
sanitizer 实例，防止 hook 跨窗口或跨消费者泄漏。`base/browser/markdownRenderer.ts` 拥有
普通 Markdown 组件、Markdown 标签/属性 allowlist 和 URL policy。
`platform/markdown/browser/markdownPreview.ts` 负责完整文档解析、预览样式及 iframe 链接
消息桥接。`workbench/contrib/markdown/browser/markdownDocumentRenderer.ts` 再将平台预览
适配为 Editor Part 可持有的 `MarkdownDocumentView`，并拥有产品级链接打开回调。

`workbench/contrib/markdown/browser/markdown.contribution.ts` 是 Workbench 功能入口，由
`workbench.contribution.ts` 静态加载；该层只接入产品视图和样式，不重复解析器或 sanitizer。
解析器返回的 HTML 从不视为可信内容，也不得绕过 DOMPurify 直接写入 DOM 或
`WebviewElement.setHtml()`。

当前 allowlist 覆盖标题、段落、列表、表格、代码块、引用和任务复选框等标准 Markdown
结构，拒绝脚本、事件属性、内联样式、SVG/MathML 与未知元素。链接只保留 `http:`、
`https:` 和页内 fragment，并由宿主接管点击；图片只保留 base64 PNG、JPEG、GIF 和 WebP，
不会直接读取本地文件或请求远程资源。预览消息仍需通过 `WebviewElement` 的 source/channel
校验，并在 `MarkdownPreview` 中再次做 exact-shape validation。

当前没有语法高亮、Markdown 扩展插件、Mermaid、KaTeX、工作区相对资源 URI 映射、滚动同步
或预览状态持久化。这些属于后续能力，加入时必须继续保持“解析后统一 sanitize，再进入隔离
容器”的顺序。

## 7. Browser Capability

Electron Main 是 Browser Target 的唯一权威持有者。

### 7.1 当前实现

`BrowserViewMainService` 为每个目标创建一个 Electron `WebContentsView`，将其挂载到所属
`BrowserWindow.contentView`，并在目标关闭或窗口释放时移除并关闭 `webContents`。新目标默认
隐藏；Renderer 必须先通过 `browserView.layout()` 提交窗口内容坐标，再通过
`browserView.setVisibility()` 显示。

调用路径固定为：

```text
Workbench consumer
  → ZetaElectronRendererApi.browserView
  → ISandboxGlobals.invoke/on
  → registerTrustedIpcRoutes
  → browserViewIpcRoutes
  → BrowserViewMainService
  → WebContentsView
```

`platform/browser/common/browserView.ts` 拥有可序列化 DTO、频道和输入 validator。
`browserViewIpcRoutes()` 只做受信 IPC 绑定，`BrowserViewMainService` 拥有 target map、原生 view、
session 安全策略、导航历史和事件投影。`WebContentsView`、`WebContents`、Electron event 与
session 对象均不得跨越 IPC。

当前 URL policy 允许 HTTPS、loopback HTTP 与精确的 `about:blank`，拒绝 URL credentials、
`file:`、`javascript:` 和其他特权 scheme。每个目标使用独立临时 partition，并固定：

```text
nodeIntegration: false
contextIsolation: true
sandbox: true
webviewTag: false
无远程页面 preload
默认拒绝 permission / device permission / download / popup
```

popup 请求只以 `openRequested` 事件返回已验证 URL，不会由远程页面直接创建窗口。Renderer
可收到目标 state、加载失败、popup 请求、renderer 崩溃和关闭事件，但不能获得底层 Electron
对象。

当前限制：

- 尚无浏览器编辑器、地址栏、标签页或 DOM 容器自动布局绑定；
- 尚未实现持久 BrowserSession、下载 UI、权限提示、证书信任或 PDF 导出；
- Browser Target 目前只绑定单个 Desktop 窗口，尚未绑定 App Server connection 或 Tool Call；
- 尚未向 Rust 暴露 browser capability，也没有开放 CDP。

### 7.2 Proposed：App Server 与 Agent 浏览器能力

Desktop 后续对 Rust 暴露语义动作时，计划使用：

- `browser/observe`
- `browser/perform`
- `browser/getPdf`

该 API 尚未实现，不能描述为当前 App Server capability。实现后仍不能暴露任意 CDP method；
每个 `targetId` 必须：

- 绑定创建它的 App Server connection；
- 在 Tool Call 开始前固定；
- 关闭后返回 `BrowserTargetUnavailable`；
- 不得静默切换到另一个活动 Tab。

## 8. Desktop 提交 App Server 能力需求

Desktop 开发者在实现前提交一份符合
[`zeta-api-interface-requirements.md`](zeta-api-interface-requirements.md) 的产品接口需求。
Desktop 是需求提出方；zeta-rs 是已接受 App Server 契约的 owner。接口必须同时评估 CLI、
daemon 和远程客户端影响，不能定义为 Desktop 私有业务 API。

文档必须覆盖：

- Client → Server 方法；
- Server → Client 请求；
- Server → Client 通知；
- Resource RPC；
- Browser Target 生命周期；
- 错误码、超时、取消、幂等和顺序；
- 每个请求、成功响应和错误响应的 JSON fixture。

zeta-rs 开发者根据该文档实现 Rust DTO、dispatcher、typed client、handler、schema 和
TypeScript 生成。进程内 CLI client 与 Desktop stdio client 必须经过同一个 dispatcher。

当前已接受的方法、通知、错误码和前端可开发范围以
[`zeta-app-server-api.md`](zeta-app-server-api.md) 为准。

## 9. Rust 交付给 Desktop 的产物

每次协议交付至少包含：

- 可运行的 `zeta` 二进制；
- `zeta app-server --listen stdio://`；
- `zeta-rs/app-server-protocol/schema/types.ts`；
- `zeta-rs/app-server-protocol/schema/schema.json`；
- schema hash；
- 当前 schema fixtures；
- Rust contract tests；
- API 变更说明。

## 10. Desktop 验收

Desktop 完成的最低证据：

- TypeScript strict build 通过；
- initialize 成功并校验 schema hash；
- Session 创建、Thread 创建/fork、订阅恢复和 Turn 中断端到端通过；
- 通知能从 App Server 到 Renderer；
- 未生成或参数错误的 IPC 被拒绝；
- 不可信网页无法访问应用 IPC；
- Browser Target 关闭后不会操作其他 Tab；
- App Server 崩溃、重启和 graceful shutdown 有测试。
