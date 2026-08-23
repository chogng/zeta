# `zeta-keybinding`

> 文档所有权：三端快捷键架构与端侧边界见 [`docs/keybindings.md`](../../docs/keybindings.md)。
> 本 README 只说明产品无关 Rust 语义核心的当前实现。

## 快速理解

`zeta-keybinding` 把标准化按键、条件和有序规则解析成命令、阻止、等待下一段 Chord 或不匹配；它也能把一份内存中的严格用户 JSON 编译成产品提供的 command/condition 类型。它不接收 DOM、winit、Crossterm 事件，不读文件，不绘制 UI，也不执行产品命令。

Zeterm 和 Zeta Code 直接依赖这个 crate；Zeta Renderer 保留同步 TypeScript 实现，并与它读取同一份 [`conformance.json`](../../resources/keybindings/conformance.json)。

## 当前所有权

| 能力 | 当前 owner | 边界 |
| --- | --- | --- |
| 逻辑键、物理键、实际与 portable modifier | `key` | adapter 必须先完成平台事件转换 |
| 一至四段 Chord、parser 与 canonical serializer | `key` / `parser` | 不定义产品默认键位 |
| `when` 表达式解析与求值 | `context` | context key catalog 和 value 由产品提供 |
| Builtin/Workbench/User、priority、注册顺序与 blocker | `binding` | 不读取用户文件 |
| 前缀、冲突与命令解析 | `resolver` | 不拥有 timeout、焦点或命令副作用 |
| 用户 JSON shape、平台覆盖与重复诊断 | `user` | 产品通过回调提供 command catalog/condition；不读取路径或替换 resolver |
| Rust/TypeScript 共同 conformance 向量 | `resources/keybindings` | 固定两端共同的语法、优先级、condition、blocker 和 prefix 子集 |

依赖方向固定为：

```text
Zeterm adapter ─┐
                ├─→ zeta-keybinding
Zeta Code TUI ──┘
```

本 crate 不得依赖 `zui`、`zeta-ui`、`zeta-winit`、Crossterm、profile 路径、App Server 或产品 command 类型。

## 公共接口

| Symbol | 职责 |
| --- | --- |
| `LogicalKey` / `PhysicalKey` / `KeyStroke` | 表示 adapter 已标准化的一次按键 |
| `Chord` / `KeySequence` | 表示一至四段有序快捷键 |
| `parse_key_sequence` / `serialize_key_sequence` | 读写共享用户配置语法 |
| `ContextExpression` | 解析并求值通用条件表达式 |
| `BindingSet` | 注册命令或 blocker 规则 |
| `KeybindingResolver` | 根据产品上下文返回 `ResolveResult` |
| `compile_user_bindings` | 严格编译完整 JSON bytes；未知 command/condition 或任一坏项使整次编译失败 |
| `UserBinding` / `UserBindingTarget` | 表示验证后的 User command 或 blocker，不携带产品副作用 |
| `user_binding_diagnostics` | 报告同 key/condition 的重复规则，并保留后声明获胜语义 |

Resolver 先过滤条件和按键前缀，再按 User/Workbench/Builtin 来源、同来源内显式 priority 和注册顺序选择获胜规则；priority 不能跨越来源层级。

## 验证

```bash
cargo test -p zeta-keybinding
cargo clippy -p zeta-keybinding --all-targets -- -D warnings
bazel test //zeta-rs/keybinding:keybinding-unit-tests
```

测试覆盖逻辑/物理键、portable modifier、parser/serializer、Unicode 空白、条件表达式、来源与极值优先级、blocker、Chord prefix、严格用户 JSON 编译和共享 conformance 向量。
