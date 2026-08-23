# `zeta-keybinding`

> 文档所有权：三端快捷键架构与端侧边界见 [`docs/keybindings.md`](../../docs/keybindings.md)。
> 本 README 只说明产品无关 Rust 语义核心的当前实现。

## 快速理解

`zeta-keybinding` 把标准化按键、条件和有序规则解析成命令、阻止、等待下一段 Chord 或不匹配；它不接收 DOM、winit、Crossterm 事件，不绘制 UI，不读配置，也不执行产品命令。

Zeterm 和 Zeta Code 直接依赖这个 crate；Zeta Renderer 保留同步 TypeScript 实现，并与它读取同一份 [`conformance.json`](../../resources/keybindings/conformance.json)。

## 当前所有权

| 能力 | 当前 owner | 边界 |
| --- | --- | --- |
| 逻辑键、物理键、实际与 portable modifier | `key` | adapter 必须先完成平台事件转换 |
| 一至四段 Chord、parser 与 canonical serializer | `key` / `parser` | 不定义产品默认键位 |
| `when` 表达式解析与求值 | `context` | context key catalog 和 value 由产品提供 |
| Builtin/User、priority、注册顺序与 blocker | `binding` | 不读取用户文件 |
| 前缀、冲突与命令解析 | `resolver` | 不拥有 timeout、焦点或命令副作用 |
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

Resolver 先过滤条件和按键前缀，再按 User/Builtin 来源、显式 priority 和注册顺序选择获胜规则。

## 验证

```bash
cargo test -p zeta-keybinding
cargo clippy -p zeta-keybinding --all-targets -- -D warnings
```

测试覆盖逻辑/物理键、portable modifier、parser/serializer、条件表达式、冲突优先级、blocker、Chord prefix 和共享 conformance 向量。
