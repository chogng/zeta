# `zeta-keybinding`

> Native 产品快捷键的跨组件路由与用户语义由
> [`docs/native-terminal-ui.md`](../../docs/native-terminal-ui.md) 维护。本 README 只拥有
> 快捷键子系统的按键模型、规则解析、录制状态和设置呈现契约。

`zeta-keybinding` 把已标准化的平台按键与有序规则解析为命令、阻止或等待后续按键，并提供
设置浮层、最多四段 Chord 的录制 lifecycle 和等待提示。它不读取窗口事件、不执行产品命令，
也不持久化用户配置。

## 所有权

| 能力 | Owner | 状态 |
| --- | --- | --- |
| 逻辑键、物理键、实际 modifier 与 portable primary modifier | `key` | ✅ |
| 一至四段 `KeySequence` 及前缀匹配 | `KeySequence` | ✅ |
| portable 字符串 parser、持久化 serializer 与 keycap label formatter | `parser` | ✅ |
| `when` 布尔/字符串表达式的解析与求值 | `ContextExpression` | ✅ |
| Builtin/User 来源、显式 priority 与注册顺序 | `BindingSet` | ✅ |
| context 过滤后的确定性冲突解析 | `KeybindingResolver` | ✅ |
| 设置浮层、命令行、深灰 keycap 与 Chord 提示 | `settings` | ✅ |
| 录制、取消、一秒 quiet period 与 commit | `recording` | ✅ |
| keycap 基础几何与 scene primitive | `zeta-ui` | 委托 |
| modal scope、稳定 identity 与 accessibility node | `zeta-ui-dispatch` | 委托 |
| winit/browser/terminal 事件转换 | 产品或平台 adapter | ❌ |
| context key 的定义和状态 | 产品 host | ❌ |
| command 注册、可用性与执行 | 产品 command owner | ❌ |
| Resolver Chord timeout 与 IME lifecycle | 窗口 host | ❌ |
| 用户配置读取、schema 和热更新 | 产品配置 authority | ❌ |

依赖方向固定为：

```text
product host → zeta-keybinding → zeta-ui
                                  zeta-ui-dispatch
```

本 crate 不得依赖 `zeta-winit`、terminal、workspace、session、editor 或产品配置 domain。
如果设置或录制模块开始定义产品 command、读取配置路径或转换平台事件，说明 ownership 已经漂移。

## 接口地图

| Symbol | 职责 |
| --- | --- |
| `LogicalKey` / `PhysicalKey` | 保存标准化逻辑身份或 adapter 提供的稳定物理代码 |
| `ShortcutModifiers` | 声明 portable primary 或明确 Control/Meta，并在解析时按 host 展开 |
| `KeyStroke` | 保存 adapter 已标准化的一次实际按键 |
| `KeySequence::new` | 拒绝空序列和超过四段的序列 |
| `parse_key_sequence` | 把空格分隔的 Chord、portable modifier 和 `[PhysicalCode]` 编译为 `KeySequence` |
| `serialize_key_sequence` | 把按键序列写回平台无关的用户配置语法 |
| `keycap_labels` / `format_key_sequence` | 按 host 生成分组 keycap label 或单行文本 |
| `ContextExpression::parse` / `evaluate` | 解析 `!`、`&&`、`||`、括号及 `==`/`!=`，由 host 回调提供 context value |
| `BindingSet::register_command` | 添加一个带来源、条件与显式优先级的命令规则 |
| `BindingSet::register_blocker` | 添加一个消费按键但不执行命令的覆盖规则 |
| `KeybindingResolver::resolve` | 由 host 提供 context predicate，返回 `ResolveResult` |
| `KeyboardShortcutsState<Command>` | 保存设置显隐、当前录制、quiet-period deadline 和保存状态，不执行或持久化命令 |
| `KeyboardShortcutsState::record` / `advance` | 接收 adapter 已标准化的 `Chord`，到期后返回 `ShortcutCommit<Command>` |
| `KeyboardShortcutRow<Command>` | 绑定宿主 command、label、稳定 `ElementId` 和当前快捷键 |
| `KeyboardShortcutsIds` | 由宿主分配 parent/root/close identity，避免组件制造全局冲突 |
| `KeyboardShortcuts` | 注册 modal interaction tree、通过 `ComponentInspection` 上报 panel bounds 并绘制设置浮层，只消费 caller-owned rows 和 diagnostics |
| `paint_chord_hint` | 把 resolver 的 pending sequence 与 entered count 绘制为底部提示 |
| `ShortcutRecording` / `ShortcutStatus` | private；分别承载录制 deadline 和保存提示，不越过 commit 边界 |

解析顺序固定为：先过滤 context 和按键前缀，再比较 User/Builtin 来源、显式 priority 和注册顺序。
获胜规则仍有后续 chord 时返回 `PendingChord`；完整 blocker 返回 `Blocked`。

```text
platform event
  → product adapter → KeyStroke
  → BindingSet + product context predicate
  → KeybindingResolver::resolve
  → NoMatch / PendingChord / Command / Blocked
  → product host owns lifecycle and execution

standardized Chord → KeyboardShortcutsState::record
                   → KeyboardShortcutsState::advance
                   → ShortcutCommit
                   → host validation / persistence
```

## 接入与失败语义

- Adapter 必须提供稳定的物理 key code，不能把键盘布局标签当成物理身份。
- `ContextExpression` 只拥有通用表达式语法；可用 context key、当前 value 和未知 key 的接受策略仍由产品 host 拥有。
- `Command` 结果只表示输入匹配，产品执行前仍需检查业务不变量。
- `PendingChord` 不启动 timer 或修改 IME；窗口 host 必须拥有这些副作用。
- Escape 和窗口失焦如何映射到 `cancel_recording` 由宿主决定；录制模块只提供 transition。
- 宿主必须为设置 root、close 和每个 command row 提供跨 frame 稳定且互不冲突的 `ElementId`。
- `ShortcutCommit` 不表示保存成功；宿主完成校验和写入后必须调用 `saved` 或 `save_failed`。
- 用户配置替换应先构造完整的新 `BindingSet`，验证成功后再原子替换。

## 测试与当前限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-keybinding
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-keybinding --all-targets -- -D warnings
```

测试覆盖 portable modifier、逻辑/物理身份、字符串 parser/serializer、host keycap label、
context 表达式、来源与顺序优先级、blocker、Chord prefix、录制/取消、modal interaction 和
设置绘制。当前没有键盘布局本地化 label、动态 registration disposable、设置搜索、分类或
恢复默认值；冲突诊断、用户配置 schema、产品 context value 和 command executor 继续由产品
host 拥有。
