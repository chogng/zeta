# `zeterm` 应用根迁移计划

> 状态：当前迁移计划。本文拥有 `zeterm` 从 `zeta-rs/native` 迁移到仓库根 `zeterm/` 的阶段、边界和验收条件；
> 共享 Rust backend 的 crate contract 由对应 `zeta-rs` README 维护，Native UI framework 的迁移细节由
> [`ui-component-migration-plan.md`](ui-component-migration-plan.md) 维护。

## 快速理解

`zeta-rs` 作为 `zeta-ts` 和 `zeterm` 共用的 Rust backend；`zeterm/` 作为 `zeterm` 的独立产品宿主；
`zeta-ts/` 继续作为 Electron 宿主。当前 `zeta-rs/native` 把产品宿主、Native UI framework 和 GPU
接线放在共享 Rust workspace 中，边界已经不再清晰。本计划先把宿主整体迁到 `zeterm/`，再把只服务
Native UI 的 crate 从共享 workspace 中分离。

| 读者关心的对象 | 当前路径 | 目标 owner | 迁移状态 |
| --- | --- | --- | --- |
| `zeterm` binary、窗口生命周期、平台事件 | `zeta-rs/native` | `zeterm/` | 阶段一迁移 |
| `zeterm` Root/Shell/Workspace 产品布局 | `zeta-rs/native/src` | `zeterm/layout` + `zeterm/composer` + `zeterm/src` | Root/Workspace 与 Composer panel/list geometry 已抽取，Shell/Composer state/scene composition 仍在宿主 |
| 通用 icon asset contract | 旧 Native icon types | `zeterm/icon` (`zeta-icon`) | 已完成；产品 catalog 保留在 `zeterm/icons` |
| Element、Scene、Interaction、Animation、Retained Runtime | `zeterm/zui` | zeterm-owned crates in root workspace | 已迁入 zeterm |
| Button、Tree、List、Editor/Sidebar presentation | `zeterm/ui`、`editor`、`agent-sidebar` | zeterm-owned crates in root workspace | 已迁入 zeterm |
| Renderer、wgpu、winit | `zeterm/renderer`、`wgpu`、`winit` | zeterm-owned crates in root workspace | 已迁入 zeterm |
| App Server、Core、Protocol、Session、File/Git、Diff、Terminal model | `zeta-rs/*` | `zeta-rs` | 保留 |
| 纯 Rust editor transaction、syntax、language service | `zeta-rs/editor-core`、`syntax`、`language-service` | `zeta-rs` | 保留；presentation 与底层分离 |

`zeta-rs` 的“共享”按宿主无关的 Rust 语义和 backend contract 判断，不要求 Electron TypeScript 直接
链接 Rust crate；Electron 通过 App Server protocol 使用同一套 authority。只有产品布局、窗口、GPU、
Native interaction composition 和平台生命周期不能继续进入 `zeta-rs` shared layer。

## 目标结构

迁移完成后的物理结构为：

```text
zeta/
├── zeterm/
│   ├── Cargo.toml              # zeterm application package
│   ├── README.md
│   ├── src/                    # zeterm product host and composition root
│   ├── zui/                    # backend-neutral native UI framework crate
│   ├── icon/                   # renderer-independent icon asset contract
│   ├── layout/                 # zeterm product pane topology
│   ├── composer/               # zeterm Composer panel/list geometry
│   ├── zui-demo/               # product-independent framework smoke host
│   ├── ui/                     # native-only reusable components
│   ├── renderer/               # native rendering crate
│   ├── wgpu/                   # native GPU backend crate
│   ├── winit/                  # native platform adapter crate
│   └── ...                     # other native-only presentation crates
├── zeta-rs/
│   ├── ...                     # shared Rust backend crates
│   ├── core/
│   ├── app-server*/
│   ├── protocol/
│   ├── editor-core/
│   ├── terminal/
│   └── ...
└── zeta-ts/                    # Electron product host
```

依赖方向固定为：

```text
desktop ───────→ zeta-rs protocol/App Server
zeterm/zeterm ────→ zeta-rs backend/domain
             └─→ Native UI workspace
zeta-rs ───────→ no zeterm/desktop product host
```

## 阶段与当前状态

### 阶段零：冻结旧 Native（已完成）

- [x] 在 `AGENTS.md` 中禁止向 `zeta-rs/native` 新增能力；
- [x] 将 Native split scene/interaction host API 迁移到 `UiFrame`，并删除旧兼容入口；
- [x] 明确 `zui`、`zeta-ui`、renderer 和生命周期机制的长期 owner；
- [x] 迁移期间将 `zeta-rs/native` 作为只读迁移源，期间不再把它当作新功能落点；阶段一完成后已删除。

### 阶段一：迁移 `zeterm` 产品宿主（已完成）

- [x] 在仓库根建立 `zeterm/` 产品目录，并以 `zeterm` 作为发布 binary；
- [x] 将当前 `zeta-rs/native` 的产品宿主、测试和 README 整体迁移到 `zeterm/`，保留现有模块边界和行为；
- [x] 将 zeterm 的依赖改为明确的 `zeta-rs` path dependency，不让 `zeta-rs` 反向知道 zeterm；
- [x] 从 shared crate 集合移除 `native` member 和产品入口；
- [x] 更新 `just zeterm`、package/build 入口和产品线文档；
- [x] 阶段完成后删除旧 `zeta-rs/native` 路径，不保留两个可运行宿主。

阶段一先解决产品宿主和共享 backend 的边界；阶段二继续把 Native-only crate 物理移动到
`zeterm/` 的直接子目录。现在由仓库根 Cargo workspace 统一解析所有 crate，zeterm 仍作为独立 package 和发布
边界存在；Bazel 使用同一个 root graph。

### 阶段二：分离 Native UI workspace（核心迁移已完成）

- [x] 将 `zui`、`zeta-ui`、`zeta-renderer`、`zeta-wgpu`、`zeta-winit` 迁入 `zeterm/` 的直接子 crate；
- [x] 保留 `zeta-editor-core`、`zeta-syntax` 等纯 Rust core 在 `zeta-rs`，将 `zeta-editor` presentation 迁入 zeterm；
- [x] 将 `zeta-agent-sidebar`、Native settings UI 和 Native keybinding UI 迁入 zeterm-side crates；
- [x] 将 Markdown presentation crate 迁入 zeterm-side crates，Theme manifest/resolver 保留在 shared backend；
- [x] 保证 `zeta-rs` backend crates 不再直接依赖 Native UI crate。

当前根 `Cargo.toml` 是唯一 workspace，`zeterm/Cargo.toml` 是 `zeterm` package。`zeterm/BUILD.bazel` 已
提供 `//zeterm:zeterm`、`//zeterm:zeterm_sources`、`//zeterm:zeterm_release_inputs`、
`//zeterm:zeterm_package_contract` 和 `//zeterm:zeterm_ci`；所有 zeterm/shared crates 通过单一 `@crates`
hub 消费 rules_rs 生成的 package deps。`bazel build //zeterm:zeterm` 已在当前 macOS toolchain 通过；
此前由 apple-cf Swift bridge 引起的限制已随 zui 的字体目录实现迁移而消除。

### 阶段三：收敛应用组合根

- [x] `zeterm/src` 只保留窗口/平台适配、产品状态投影、command 执行和具体 Part/Overlay 组合；
- [x] `ShellPresentation` 使用单一 `zui::UiFrame` owner；
- [x] 清零 split scene/interaction API 的生产调用并删除旧入口；
- [x] retained fragment cleanup、animation deadline 和 redraw invalidation 全部从 `zui::RetainedRuntime`/
      `AnimationRegistry` 派生；具体产品 fragment 是否采用 exit retention 仍由产品状态决定；
- [x] 删除剩余 `zeta-native` 命名、旧 build target 和旧文档引用；迁移历史中的旧名称只保留在迁移历史、
      AGENTS 约束和边界检查器中。

### 阶段四：独立产品发布

- [x] `zeterm` 能独立构建、测试，并通过 package staging 生成带 binary digest 的 unsigned `zeterm` artifact；
- [x] 为 `zeterm` 建立发布输入和 boundary CI graph；`//zeterm:zeterm_ci` 验证 workspace ownership，
      `//zeterm:zeterm_release_inputs` 固定 package/signing contract；Cargo workspace 仍是唯一 canonical
      Rust build graph；
- [x] 建立 `zeterm/` 与 `zeta-rs` shared crates 的 metadata-derived hermetic Bazel Rust graph；
      单一 root workspace 已消除跨 workspace path dependency bridge；
- [x] 在 CI 入口中通过完整 `//zeterm:zeterm` build 验证平台 toolchain；本机已通过同一 target，
      `//zeterm:zeterm_ci` 继续验证 boundary/package/signing contract。
- [x] 接入 provider-neutral 的 platform signing/verification job；`build/release/release_zeterm_package.sh`
      负责 Build → Stage → Sign → Verify，平台 CI 只需绑定 native tool 和 secret 环境；只发布
      `verified` artifact；
- [x] `zeta-ts` 构建不拉入 Native UI/GPU 依赖；desktop 侧只保留自身的 platform adapter，边界检查未发现
      对 zeterm UI/GPU crate 的依赖。
- [ ] `zeta-rs` backend 可独立测试、发布和被多个宿主复用；
- [ ] App Server protocol、theme manifest 和 shared domain revision 具备明确兼容策略；
- [x] packaging、CI、开发命令和文档只引用新的 product roots。

### 阶段五：通用框架可移植性（当前演进）

- [x] 将 `IconId`、SVG definition 和 rendering mode 下沉到独立的 `zeta-icon` contract；`zeta-icons`
      只保留可选的 zeterm product catalog，`zui`/`zeta-ui` 不再依赖该 catalog；
- [x] 将 Root/Inspector 和 Terminal Workspace 的 pane topology 抽到 `zeta-layout`，Native 只投影
      `AgentSidebarState` 为 `SidebarLayoutSpec`；
- [x] 建立 `zui-demo`，只依赖 `zui`、`zeta-ui` 和 `zeta-renderer`，以 recording backend 验证通用
      组件可脱离 zeterm product host 组合；
- [x] 将 Composer panel、interaction list 与 selection scroll geometry 抽到 `zeta-composer`；Native
      只投影 item count、preferred height 与 scene/state adapter；
- [ ] 将更多 Shell/Composer/Session domain composition 按 owner 分批抽到产品领域 crate；不把产品
      state、command 或平台事件下沉到 `zui`；
- [ ] 在拥有第二个真实宿主前，不拆分 `zui` 的内部 foundation/layout/presentation/runtime 为更多
      独立 crate；先保持单一 framework contract，降低跨 crate API churn。

### 当前审计结论（2026-08-03）

物理迁移已经完成：旧 Native UI 目录、旧 workspace、旧产品 build target 和 shared backend 到 UI 的
反向依赖均已清除；根 Cargo workspace、Bazel target graph、`zeterm` binary build、targeted tests 和
boundary CI 均已通过。

剩余两个未勾选项是产品演进工作，不是迁移残留：一是为 shared backend 建立独立发布 artifact/CI
流程，二是为 App Server、theme manifest 和 shared domain revision 定义跨版本兼容策略。它们可以在
当前迁移后的边界上继续建设，不应再把 UI 从 `zeta-rs` 搬迁作为前置条件。

## 迁移约束

- 不复制 `zeta-rs/native` 形成第二个并行宿主；迁移必须保持单一运行入口。
- 不在旧 Native 中修建新功能；若迁移过程中发现缺少通用能力，先在正确的下层 owner 实现，再由
  zeterm 做最小接线。
- 不把 `zeterm/editor`、`zeterm/agent-sidebar` 等 presentation crate 直接误判为 shared backend；
  先按“headless model/core”和“Native presentation”拆分。
- 不让 `zeterm` 类型、产品命令、窗口事件或布局类型进入 `zeta-rs`。
- 每个迁移阶段都必须保留 deterministic unit tests 和至少一个产品 targeted test；测试失败时先区分
  迁移回归与工作区已有的无关 dirty change。

## 验收入口

阶段一完成后至少通过：

```text
cargo check --manifest-path Cargo.toml --workspace
cargo check --manifest-path Cargo.toml -p zeterm
cargo test --manifest-path Cargo.toml -p zeterm
cargo test --manifest-path Cargo.toml --workspace
just --dry-run zeterm
```

`just --dry-run zeterm` 只验证入口已经指向根 workspace 的 `zeterm` package；真正启动窗口时使用
`just zeterm`。根 workspace 的 zeterm package 与 shared backend 已通过 workspace check 和 targeted
check/test 验证；Bazel graph 的完整入口是 `bazel build //zeterm:zeterm`，发布 contract 入口是
`bazel test //zeterm:zeterm_ci`，package staging 使用 `just zeterm-package`，
签名发布流程使用 `build/release/release_zeterm_package.sh`，发布细节见
[`zeterm-release-graph.md`](zeterm-release-graph.md)。

并满足以下结构检查：

- `rg 'zeta-rs/native|zeta-native'` 只剩迁移历史、兼容说明或明确的旧版本记录；
- 根 `Cargo.toml` 是唯一 Cargo workspace；`zeterm/Cargo.toml` 不声明 nested workspace；
- `zeterm` 只向下依赖 `zeta-rs` shared crates 与 zeterm-owned UI crates；
- 任何 shared crate 都不依赖 `zeterm`、窗口实例、GPU handle 或产品 layout。
