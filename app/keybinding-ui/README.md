# `app-keybinding-ui`

> 文档所有权：三端快捷键架构见 [`docs/keybindings.md`](../../docs/keybindings.md)，App 输入和产品 UI 边界见 [`native-terminal-ui.md`](../docs/native-terminal-ui.md)。
> 本 README 只说明 App 快捷键设置与录制 UI 的当前实现。

## 快速理解

`app-keybinding-ui` 拥有 App 的快捷键设置浮层、Chord 录制状态和提示绘制；它消费 [`zeta-keybinding`](../../zeta-rs/keybinding/README.md) 的标准按键值，但不解析平台事件、不读取配置文件，也不执行命令。

## 当前所有权

| 能力 | 当前 owner | 边界 |
| --- | --- | --- |
| 最多四段 Chord 的录制、取消和 quiet-period commit | `KeyboardShortcutsState` | timer 推进和保存结果由 App host 接线 |
| 设置行、modal interaction、keycap 和诊断绘制 | `KeyboardShortcuts` | 产品提供稳定 identity、命令行和当前规则 |
| pending Chord 底部提示 | `paint_chord_hint` | Resolver 与 Chord lifecycle 不在本 crate |
| 通用按键与规则解析 | `zeta-keybinding` | 委托 |
| 用户配置读取、校验和原子替换 | `zeta-keybindings-host` / App resource | 委托 |
| 产品命令执行、焦点、IME 和窗口失焦 | App | 委托 |

依赖方向固定为：

```text
App → app-keybinding-ui → zeta-ui → zui
                            └→ zeta-keybinding
```

## 验证

```bash
cargo test -p app-keybinding-ui
```

测试覆盖录制提交/取消、modal interaction、设置绘制和 pending Chord 提示。
