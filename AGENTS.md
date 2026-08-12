`../vscode` and `../codex` and `../zed` and `../warp` and `../pi`

# Communication

- When comparing responsibilities, capabilities, implementation status, or design options, prefer
  a compact, conclusion-oriented comparison table when it makes the distinction clearer.
- Use `✅` and `❌` for genuinely binary judgments so ownership and support boundaries are visually
  unambiguous.
- Do not force nuanced states into a binary marker. Use explicit labels such as `部分具备`,
  `尚未完成`, `协调`, or `委托` when those are more accurate.
- Lead with the conclusion; use surrounding prose only to explain important boundaries or caveats.

## Learnings

- 在把新能力接入 `native` 前，先按依赖方向判断其是否属于通用框架机制。帧调度、失效等级、retained presentation、Scene fragment 生命周期和局部重建策略应优先由 `zui` 提供后端无关契约；`native` 只保留产品状态映射、平台事件适配和具体 Part/Overlay 组合，不得因接入方便复制或拥有框架运行时。
- 当用户讨论专化 Workbench、Sessions 或其他产品前端概念时，不要因为 IDE 当前打开的是 `zeta-code/tui` 文件就默认把目标定位到 TUI；应先在前端 Renderer/Workbench 范围定位同名能力，只有明确提到终端、Ratatui 或 `zeta-code` 时才转向 TUI。
* 设计检索链路时，必须把 Workspace authority、模型接入与检索编排分开：所有源码扫描、ignore、读取、切块、revision 与 chunk identity 都由 Workspace 侧 CodeIndex 拥有，云端只能消费 Workspace 已授权并复核的精确 chunks，不能读取整文件后重新切块；`model-provider` 只统一调用 embedding/rerank 模型；云端 CodeIndex 服务负责准备模型输入、向量检索、调用 rerank，并依据模型分数排序、过滤和截断；跨来源结果融合才属于 retrieval 层。不得因为模型返回向量或相关性分数，就把索引策略或排序所有权归给 `model-provider` 或客户端 retrieval。
- 设计 Skill 的日常显式入口时，应把可调用 Skill 直接投影为统一斜杠面板中的动态命令（如 `/commit`）；`/skills` 只承担浏览、启用、诊断等管理职责。Skill 列表只加载元数据，完整 `SKILL.md` 仍在选中或自动激活后按需加载。

## Product ownership

- `zeta-rs/` is the shared Rust backend boundary. Shared protocol, App Server, domain, storage,
  execution, terminal semantics, and other backend-neutral crates belong there; product hosts do not.
- `zeterm/` owns the native `zeterm` product, including `zui`, `zeta-ui`, renderer, `wgpu`, and
  `winit` as direct child crates.
- `zeta-code/` owns the `zeta code` product host: `zeta-cli` and `zeta-tui`. Do not add TUI
  presentation, raw-mode lifecycle, Ratatui interaction, or CLI product composition to `zeta-rs/`.
- All three boundaries may remain members of the single root Cargo workspace; workspace membership
  does not change implementation ownership.

## Native 迁移边界

- `zeta-rs/native` 已进入弃用迁移期。后续默认不得向其中新增产品能力、通用 UI 机制、组件实现、布局算法、交互树、动画、deadline、retained lifecycle 或新的状态 owner；不要把 Native 当作新功能的落点。
- 新能力必须先落到正确的长期 owner：后端无关的 frame、layout、paint、inspection、interaction、animation、失效和 retained lifecycle 放入 `zui`；可复用 UI 组件放入 `zeta-ui`；文件、SCM、编辑器、终端等领域能力放入对应领域 crate。
- Native 只允许保留三类改动：删除或迁移旧实现的兼容改动；把平台事件、产品状态和 command 映射到下层 canonical API 的薄适配；维持现有产品宿主运行所必需的最小组合接线。任何例外都必须在改动说明中写明 owner、迁移终点和删除条件。
- 不得在 Native 新建 framework helper、第二套 registry/timer、重复的 layout 或 inspection 计算、手写 interaction registration，或为了绕过下层 API 在 Native 增加新的公共抽象。优先扩展下层 owner，再回到 Native 做最小 adapter 接线。
- 修改既有 Native 文件时，优先把代码迁出、标记 `deprecated`、缩小职责或删除；不得因为已有实现就在 Native 继续扩展同一职责。Native 的 split scene/interaction host boundary 属于迁移债务，清零后应删除。
- 如果任务看起来需要“往 Native 里写”，先停下来重新判断是否应移动到 `zui`、`zeta-ui` 或领域 crate；无法证明只是薄适配或迁移清理时，不得直接实现。

# Only for Rust Crates

- Newly added traits should include doc comments that explain their role and how implementations are expected to use them.
- Avoid bool or ambiguous `Option` parameters that force callers to write hard-to-read code such as `foo(false)` or `bar(None)`. Prefer enums, named methods, newtypes, or other idiomatic Rust API shapes when they keep the callsite self-documenting.
- Prefer private modules and explicitly exported public crate API.
- Prefer file-based Rust module roots over `mod.rs`: use `foo.rs` for the `foo` module and `foo/bar.rs` for its child modules. The parent module should compose private children and explicitly re-export the public API; implementation details belong in named child modules. Do not introduce new `foo/mod.rs` files. Keep an existing `mod.rs` only when compatibility, generated code, or an external layout constraint requires it, and migrate it to `foo.rs` when the module is substantially modified.
- Prefer one Rust import per line over brace-grouped imports. For example, prefer `use foo::Bar;` and `use foo::Baz;` over `use foo::{Bar, Baz};`.
- Avoid large modules:
  - Prefer adding new modules instead of growing existing ones.
  - Target Rust modules under 500 LoC, excluding tests.
  - If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending the existing file unless there is a strong documented reason not to.
  - When extracting code from a large module, move the related tests and module/type docs toward the new implementation so the invariants stay close to the code that owns them.

## Documentation

- Follow [`docs/documentation-guidelines.md`](docs/documentation-guidelines.md).
- Crate READMEs should focus on implementation ownership, exact contracts, execution paths,
  failure semantics, integration obligations, tests, modification impact, current limitations, and
  crate-level extension points.
- Crate READMEs should name the key private symbols that carry ownership, validation, binding,
  failure semantics, and extension direction. Include their real call relationships and identify
  internal changes that would signal architectural drift.
- `docs/*.md` should focus on cross-crate architecture, product semantics, ownership, tradeoffs,
  trust and durability boundaries, current system status, and staged evolution.
- Keep current implementation, proposed work, and potential future directions explicitly
  separated. Do not describe future capability as current behavior.
- When a crate README and a system document cover the same topic, make their canonical ownership
  explicit, link them in both directions, and avoid duplicating the same detailed explanation.

## Tests

### zeterm command-line testing

- `zeterm` 的功能、交互、session/tab、terminal pane、PTY 生命周期和性能回归测试必须以命令行可执行的测试为准；不得使用截图、图像快照或像素比对作为通过/失败依据。
- 优先为状态机、command dispatch、semantic element identity、事件流、projection、PTY I/O 和 pane 生命周期建立 Rust 单元测试或集成测试；测试应断言状态、事件、时序、输出和耗时等可观测结果，而不是屏幕像素。
- 需要验证运行中的 zeterm 时，使用 `cargo run -p zeterm` 或构建后的二进制配合命令行 trace；可使用 `ZETERM_SESSION_TRACE=1`，必要时再使用 `ZETERM_SESSION_TRACE_FRAMES=1` 观察 session 切换、terminal ready、重建和渲染耗时。
- GUI 手工操作只能作为补充验收，不能替代命令行回归测试；不能可靠注入到 `wgpu` canvas 的鼠标坐标、截图或图像结果不得被报告为测试通过。
- 新增 UI 测试时，应先提取可在无窗口环境运行的纯逻辑或语义事件测试；不要为了截图测试引入显示器、窗口或图像基线依赖。

### Test module organization

- When adding a new test module, define its contents in a separate sibling file rather than inline in the implementation file.
- Use an explicit `#[path = "..._tests.rs"]` attribute so the test filename is descriptive and easy to locate:

  ```rust
  #[cfg(test)]
  #[path = "parser_tests.rs"]
  mod tests;
  ```


# Only for TypeScript

## Interface design

- Design from the caller's point of view: common usage should be concise,
  natural to read, and independent of implementation details.
- Keep interfaces small, complete, and canonical. Every method, option,
  overload, alias, and abstraction must add distinct semantic value.
- Use domain language and the type system to make intent clear and invalid or
  ambiguous calls difficult to express.
- Prefer standard protocols when they already produce clear, interoperable call
  sites.
- Validate interfaces with representative usage. Prefer clear code over clever
  compression or explanatory comments.

## Service boundaries and naming

- Treat service alignment as a repository-wide architecture rule, not a feature-specific convention.
- Put a frontend domain service contract in a `common/*Service.ts` file, and name its public interface
  and service identifier `I<Capability>Service`.
- Name each runtime implementation file after its exported implementation class, including the runtime
  qualifier when it matters, such as `appServerSyntaxAnalysisService.ts` exporting
  `AppServerSyntaxAnalysisService`.
- Keep transport APIs and generated DTOs inside runtime implementation modules. Product consumers must
  depend on the frontend service contract and frontend-owned domain types.
- Align capability names, operation semantics, lifecycle, and error categories across the frontend
  service, transport protocol, and backend service so adapters stay thin and mechanical.
- Name adapter and test files after the contract or implementation they exercise. Do not preserve an
  obsolete transport, editor, or host name after ownership has moved.

## Base module boundaries

- Reverse dependencies from `src/zeta/base` into any higher-level domain are
  strictly prohibited. Features such as PDF, editors, workspaces, sessions, or
  file explorers may depend on base APIs; base modules must never import,
  reference, specialize for, or otherwise depend on those features.
- Keep modules under `src/zeta/base` domain-agnostic. Higher-level domains must not
  determine base interfaces, types, defaults, comparison rules, lifecycle
  behavior, tests, or examples.
- Define URI parsing, URI identity, resource collections, UUID validation, and
  lifecycle primitives in terms of their general contracts rather than a
  current feature's needs or examples.
- Do not make a general resource comparison rule silently ignore URI
  components for one consumer. Preserve exact URI identity by default and let a
  domain explicitly select alternate semantics, such as ignoring fragments.
- Keep domain identities and lifecycle rules, including document IDs and editor
  instance IDs, in the module that owns those concepts. Do not introduce them
  into `src/zeta/base` before a concrete domain model requires them.
- Add structures such as a resource tree when a real hierarchical consumer
  exists. Do not expand the base layer speculatively from anticipated feature
  requirements.

## Renderer UI styling ownership

- Follow [`docs/ui-styling-ownership.md`](docs/ui-styling-ownership.md) for every
  Renderer component, Workbench Part, contribution, theme, and CSS change.
- Before adding or changing a visual rule, identify the owner from the state
  definition, DOM creator, and hosting boundary. The owner must keep its internal
  geometry and interaction-state styles with the component that defines them.
- Follow the VS Code state-projection convention: DOM state must expose a stable
  class such as `.checked` alongside the corresponding ARIA attribute. CSS must
  select the state class rather than using ARIA attributes as visual selectors.
- Workbench Part CSS may own region layout, borders, backgrounds, and the external
  box of a directly hosted component. It must not reach through shared component
  internals to override item, hover, active, focus, selected, or disabled styles.
- Express legitimate visual differences through a named presentation variant or
  semantic token owned by the component. Do not introduce host-specific deep
  selectors or ambiguous boolean styling options.
- Treat historical selectors that violate the canonical document as migration
  debt, not precedent. When modifying an affected area, move the rule to its owner
  or add the required public presentation contract.

## Code formatting

- Every TypeScript import declaration must occupy exactly one physical line. Never wrap imported names or any other part of an import declaration across multiple lines.
- Prefer compact single-line formatting for other short function calls, parameter lists, conditions, and expressions.
- Do not preemptively use multiline formatting merely because more items might be added later.
- Use TypeScript `private` or `protected` members instead of ECMAScript `#private` identifiers so internal call sites read as `this.member`.
- Prefix a private backing member with `_` only when it must coexist with a public member of the same semantic name, such as `_onDidChange` backing `onDidChange`.
