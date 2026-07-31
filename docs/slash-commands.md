# Slash Commands 架构

> 本文拥有 Slash Commands 的跨产品语义、运行时边界与当前接入状态。Rust 实现细节由
> [`zeta-slash-commands` README](../zeta-rs/slash-commands/README.md) 拥有；App Server wire snapshot
> 由 [`zeta-app-server-api.md`](zeta-app-server-api.md) 拥有。

## 结论

Slash Commands 是一个无渲染产品能力，不是 TUI feature，也不是用户 config。App Server 在
`initialize.slashCommands` 发布 server commands；每个 client 再与自身真正可执行的 local commands
合并。命令定义、名称冲突、输入 grammar、匹配与选择都先于 renderer，只有行布局、DOM/WGPU/Ratatui
绘制和平台输入事件留在三种 renderer。

| Surface | Catalog 来源 | Core/adapter | Renderer owner |
| --- | --- | --- | --- |
| TUI | built-ins + initialize snapshot | 直接使用 `zeta-slash-commands` | Ratatui popup |
| Native zeta-ui | local `/model` + initialize snapshot | 直接使用 `zeta-slash-commands`；Native 另拥有 model picker | WGPU composer interaction rows |
| Desktop Chat | Workbench actions + initialize snapshot | canonical generated `SlashCommandDefinition` + separate action binding | Alpha completion widget；textarea/Monaco 可复用同一 catalog |

## 所有权与执行

```text
App Server composition
  → initialize.slashCommands (immutable server snapshot)
  → client merges executable local commands
  → validated SlashCommandCatalog
  → input/query/matches/selection/completion
  → renderer projection
  → activation
      local  → client product command
      server → unchanged /name text in turn/start.input
```

`SlashCommandDefinition` 是三端唯一的 Slash Command model。`origin`、Workbench `actionId` 和 TUI
dispatcher identity 都是与 model 分离的 client binding，不能包装成另一种 Slash Command。

App Server 不执行 server-advertised Slash Command；它只声明 discoverability 与参数能力。Local command
必须存在真实 client execution path，否则不能进入 catalog。Desktop 的 `/new`、`/history` 属于
Workbench command mapping；Native 的 `/model` 属于 Session model selector；TUI built-ins 属于 TUI
coordination。任意 local/server 同名都拒绝整份合并结果，不按客户端优先级静默覆盖。

## Config 边界

`slashCommands` 不进入通用 config，也不由 slash command view 读取 config。它是 connection 初始化时
冻结的 capability snapshot。未来若 config 决定某类命令是否可用，应由 composition 根据 config 生成
下一条 connection 的 snapshot；已初始化 client 不观察中途变化。

## 当前限制

Rust surfaces 直接共享 `zeta-slash-commands` 的 headless state。Desktop 直接消费同一个 generated
`SlashCommandDefinition` model，并由 Alpha Editor 的通用 completion/session state 投影交互；TypeScript
只保留运行时 catalog binding。Rust crate 与 Desktop adapter 共同执行
`zeta-rs/slash-commands/fixtures/conformance.json`，确保名称校验、大小写、前缀匹配和参数规则一致。

## 修改影响

新增 wire 字段先修改 `zeta-app-server-protocol` 并重新生成 TypeScript/schema。修改名称、参数、匹配或
输入规则必须同时更新 crate tests、跨运行时 fixture、TUI/native adapter tests 和 Desktop tests。
纯视觉变化只修改对应 renderer，不得把 host-specific state 反推到 catalog 或 protocol。
