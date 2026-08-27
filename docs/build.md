# 构建系统、仓库脚本与输出目录

> 本文拥有 Zeta 构建入口、开发者脚本、受版本控制的构建逻辑与本地可删除产物之间的目录边界。Desktop 产品版本选择仍由 [`product-editions.md`](product-editions.md) 维护。

## 快速理解

Zeta 使用 `build/` 保存构建系统实现，使用 `scripts/` 保存开发者和 CI 直接调用的稳定操作入口，使用根 `.build/` 保存常规本地产物。日常构建、测试和开发不再向 `zeta-ts/` 或仓库根散落 `dist`、`output`、`target` 和 Bazel 便捷链接；Sites 部署协议要求的 `docs-site/dist/` 是唯一部署暂存例外，仍由统一清理入口删除。

| 看到的路径 | 它是什么 | 是否受版本控制 | 能否整体删除 |
| --- | --- | --- | --- |
| `build/` | 按机制分类的构建配置、监听器和清理入口 | 是 | 否 |
| `scripts/` | 测试、诊断和维护等稳定仓库操作入口 | 是 | 否 |
| `.build/` | Cargo、Desktop、测试和 Bazel 本地产物 | 否 | 是，运行 `corepack pnpm clean` |
| `docs-site/dist/` | Sites 要求的文档站部署暂存目录 | 否 | 是，运行 `corepack pnpm clean` |
| `zeta-ts/generated/` | 协议和图标生成后参与编译的源码 | 部分文件按生成规则管理 | 否，必须由对应同步命令更新 |
| `zeta-ts/docs/`、`zeta-ts/licenses/` | Desktop 的文档和打包输入 | 是 | 否 |
| `node_modules/` | pnpm workspace 的依赖链接和虚拟依赖树；内容寻址 store 使用用户级默认缓存 | 否 | 可通过 `corepack pnpm install` 重新安装 |
| `.zeta/` | 当前工作区的 Zeta 配置或运行状态 | 按工作区用途决定 | 不应由构建清理 |

## 构建入口

根 `package.json` 提供统一入口；产品特定命令继续由根命令委托到对应构建系统。

| 命令 | 结果 |
| --- | --- |
| `corepack pnpm build` | 依次构建 Electron Desktop 和根 Cargo workspace |
| `corepack pnpm build:desktop` | 构建 Electron Main、Preload 和当前 `ZETA_PRODUCT` Renderer |
| `corepack pnpm build:docs` | 生成文档数据并构建文档站部署产物 |
| `corepack pnpm check:docs` | 独立运行全仓文档规范检查，不阻塞本地站点打包入口 |
| `corepack pnpm build:rust` | 构建根 Cargo workspace |
| `corepack pnpm test:build` | 运行构建工具自身的单元测试 |
| `corepack pnpm test` | 通过 `scripts/test.ts` 运行构建工具检查和 Desktop 单元测试 |
| `corepack pnpm test:integration` | 通过 `scripts/test-integration.ts` 运行 Editor 浏览器集成测试 |
| `corepack pnpm test:web-integration` | 通过 `scripts/test-web-integration.ts` 运行带 App Server 的完整 Web 集成测试 |
| `corepack pnpm test:documentation` | 通过 `scripts/test-documentation.ts` 构建并测试文档站 |
| `corepack pnpm test:desktop:smoke` | 通过 `scripts/test-smoke.ts` 运行 Electron Desktop smoke tests |
| `corepack pnpm typecheck:build` | 严格检查整个 `build/` 中的 TypeScript 构建代码 |
| `corepack pnpm typecheck:scripts` | 严格检查整个 `scripts/` 中的 TypeScript 仓库脚本 |
| `corepack pnpm typecheck:docs` | 检查文档站 TypeScript |
| `corepack pnpm clean` | 删除 `.build/`、Sites 部署暂存和已知旧输出，不删除依赖、工作区状态或源码生成物 |

Desktop 的 `code` 与 `academic` 仍通过同一个 `build:desktop` 入口构建；`ZETA_PRODUCT` 只选择矩阵项，不创建另一套命令。

## 输出布局

```text
.build/
├── cargo/                       # Cargo target-dir
├── desktop/
│   ├── main/                    # Electron Main TypeScript
│   ├── preload/                 # sandbox Preload TypeScript
│   ├── renderer/<product>/      # Vite Renderer bundle
│   ├── node_modules/            # link to the single Desktop dependency install
│   ├── test/                    # compiled Node tests
│   ├── editor-browser/          # editor browser-test bundle
│   ├── playwright/              # Playwright results and reports
│   └── dev/
│       ├── zeta-package/        # validated development App Server package
│       ├── server-host/         # hot-reload generations
│       └── web-profile/         # full Web development profile
└── bazel-*                      # Bazel convenience links
```

文档站的 TypeScript 增量状态、Wrangler 日志和 Miniflare 注册表写入 `.build/docs/`；`docs-site/dist/` 仅保留给 Sites 部署工具，因为该协议固定从站点项目读取 `dist`。

`.cargo/config.toml` 把默认 Cargo `target-dir` 固定为 `.build/cargo`；显式 `CARGO_TARGET_DIR` 仍可覆盖它。`.bazelrc` 只把工作区便捷链接放入 `.build/`，Bazel 自己的输出用户根仍由 Bazel 管理。

根 Cargo profile 在 `dev` 与 `test` 中对完整依赖图使用轻量优化，并对 `app`、`zeta-app-server` 与 `zeta-app-server-client` 的超大最终链接单元使用 size optimization；debug assertions、各 profile 既有的调试信息与增量编译仍然保留。该配置把 macOS 产物的 `__eh_frame` 控制在 compact-unwind 的 16 MiB 编码上限内，不能用关闭 `linker_messages` 代替。

## 构建源码与仓库脚本边界

`build/` 沿用 VS Code 的机制分类，而不是按产品复制工具链。只为已经存在的构建职责创建目录：

| 路径 | 单一职责 |
| --- | --- |
| `build/lib/` | 构建输出路径、Desktop 输出准备等共享基础设施 |
| `build/lib/watch/` | Electron TypeScript 与 Rust Server Host 的增量监听和重启协调 |
| `build/pnpm/` | pnpm 版本约束、安装入口和单锁文件 workspace 校验 |
| `build/desktop/` | Desktop 开发包、资源生成、Electron 启动和打包校验 |
| `build/docs/` | 文档规范检查与文档站数据生成 |
| `build/vite/` | Renderer 入口、Vite 配置、开发桥接与热重载插件 |
| `build/download/` | 受锁文件约束的第三方构建运行时下载器 |
| `build/app/` | app 生成器 |
| `build/release/` | Python/Shell 发布打包、签名、验证以及 Bazel 入口 |
| `build/clean.ts` | 根清理入口 |
| `build/package.json` | 构建工具自身的依赖、测试和类型检查入口 |
| `build/tsconfig.json` | 构建工具 TypeScript 边界 |

`scripts/` 使用 VS Code 式的面向操作命名，顶层文件是开发者和 CI 可直接调用的稳定入口；具体测试运行器和 Node loader 放在 `scripts/test/`。当前入口包括 `test.ts`、`test-editor.ts`、`test-extensions.ts`、`test-integration.ts`、`test-web-integration.ts`、`test-documentation.ts` 和 `test-smoke.ts`。只有形成真实、可运行的仓库操作契约时才新增入口；当前没有独立的远端 SSH 端到端测试，因此不提供空的 `test-remote-integration.ts`。

`scripts/` 可以调用 `build/` 的构建准备能力，`build/` 不得依赖或调用 `scripts/`。测试内容和 fixture 仍归对应产品目录拥有，仓库脚本只负责稳定入口、进程编排和临时测试输出生命周期。

根 `package.json`、`pnpm-workspace.yaml` 和 `pnpm-lock.yaml` 必须留在仓库根，因为它们是 pnpm 发现 workspace 和执行根命令的协议文件；安装策略与校验实现由 `build/pnpm/` 拥有。`build`、`scripts`、`zeta-ts` 和 `docs-site` 共用根锁文件与 TypeScript 版本，pnpm 内容寻址 store 使用用户级默认缓存，子项目不得再声明独立 `packageManager`、`pnpm` 策略或 npm 锁文件。

同理，`.bazelrc`、根 `BUILD.bazel`、`.cargo/config.toml` 和 `tsconfig.base.json` 是对应工具从仓库根发现的协议文件，不能为了让 `build/` 看起来更大而移动。`docs-site/vite.config.ts`、`next.config.ts`、`postcss.config.mjs`、`eslint.config.mjs` 和 `.openai/hosting.json` 是站点框架在项目根发现的适配配置；共享生成、打包和验收实现仍归 `build/docs/`。

Node 构建工具和仓库脚本统一使用可擦除语法范围内的 TypeScript（`.ts`），由当前 Node.js 直接执行，不生成一份中间 JavaScript 副本；`build/tsconfig.json` 和 `scripts/tsconfig.json` 分别通过 `erasableSyntaxOnly` 和严格类型检查保证两个源码边界同时满足 Node 运行时约束。`build/release/` 保留已有 Python/Shell 发布契约，因为它们由 Bazel 和平台签名流程直接调用，不伪装成 Node 工具。

平台专属构建流程只有在出现实际实现时才新增 `build/win32/` 或 `build/linux/`，不创建空分类。`zeta-ts/` 只保存产品源码、测试内容和产品清单；构建、资源生成、下载与发布逻辑由根 `build/` 拥有，跨产品测试和维护编排由根 `scripts/` 拥有。Renderer、Workbench 和平台服务不得拥有构建工具配置或仓库操作入口。

旧的 `target/`、`zeta-ts/dist/`、`zeta-ts/output/`、`zeta-ts/.tmp/` 和根 `output/` 仍保留忽略规则，只为防止旧工具或旧分支重新提交这些产物；当前命令不得再写入这些路径。
