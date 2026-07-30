# Native Text Input：分层、IME 与当前实现

> 状态：Current。
> 本文拥有 native 单行文本输入的跨 crate ownership、用户语义和演进边界。具体源码接口与
> 修改路径分别由 [`zeta-native`](../zeta-rs/native/README.md)、
> [`zeta-ui`](../zeta-rs/ui/README.md) 和 [`zeta-winit`](../zeta-rs/winit/README.md) 说明。

## 1. 决策摘要

Native 文本输入采用单向依赖，不建立同时理解窗口事件、编辑状态和 GPU scene 的总括 widget：

```text
winit keyboard / IME event
  → zeta-winit platform forwarding
  → zeta-native focus and event routing
  → zeta-ui TextInput foundation
  → TextInputLayoutEngine
  → InputBox component
  → UiScene
```

`zeta-ui` 不依赖 `zeta-winit`；`TextInput` 不保存 `winit::Ime`、`KeyEvent` 或 GPU 类型；平台
adapter 不理解 committed text、selection 或 composition。这个边界让 Unicode 编辑语义、
shaping geometry 和真实平台接入可以分别测试。

## 2. Ownership

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| 原生 keyboard/IME event 与候选框 API | `zeta-winit` / `winit` | 委托 |
| Composer focus、event routing、IME activation | `zeta-native::NativeApp` | ✅ |
| Committed text、selection、grapheme movement | `zeta-ui::TextInput` | ✅ |
| Preedit/commit/cancel composition state | `zeta-ui::TextInput` | ✅ |
| 单行 shaping、selection/caret/preedit geometry | `zeta-ui::TextInputLayoutEngine` | ✅ |
| Caret blink phase state machine | `zeta-ui::CaretBlinkController` | ✅ |
| Input-box chrome、状态与 scene composition | `zeta-ui::InputBox` | ✅ |
| Blink deadline scheduling 与 redraw | `zeta-native::NativeApp` | ✅ |
| Mouse caret placement、drag selection | 尚无 owner | 尚未完成 |
| Clipboard、undo/redo、accessibility | 尚无 owner | 尚未完成 |
| 多行 editor、soft wrap、vertical navigation | 尚无 owner | 尚未完成 |

`TextInput` 是非 component 基座：拥有编辑状态、composition 和 shaping contract，但不实现
`Component`，也不拥有边框、背景、placeholder 或 hover/focus 视觉。`InputBox` 才是
`Component`，它组合 `TextInput` 的 immutable layout 与自己的 chrome/style。
`CaretBlinkController` 同样不是 component；它只计算时间相位，不创建 timer 或请求 redraw。

## 3. 用户语义与不变量

- committed text 与 preedit 永远分离；`Preedit` 只改变临时 composition，`Commit` 才写入文本；
- selection、cursor 和 IME cursor range 都使用 UTF-8 byte index；
- 普通移动和删除以 extended grapheme cluster 为边界，不拆开组合附加符或 emoji sequence；
- composition 在视觉上临时替换 active selection，但不修改 committed text；只有 `Commit` 才原子
  替换 selection，cancel 保留原始文本和选择；
- IME 没有提供 preedit cursor 时隐藏 caret；提供 range 时使用 range end 定位 caret；
- composer 获得焦点时启用 IME，失焦时禁用 IME 并清除未提交的 preedit；
- 获得焦点或发生 editing/composition activity 时 caret 立即可见，之后按 deadline 切换相位；
- active selection 隐藏普通 caret；IME preedit cursor 仍遵守平台提供的 visible/hidden range；
- 候选框锚点来自 shaped caret 的 logical window coordinates，不使用字符数估算；
- 单行模型拒绝换行与控制字符。

## 4. 当前端到端路径

```text
pointer release on composer
  → ShellInteraction focuses composer
  → NativeWindow::enable_ime
  → rebuild ShellPresentation
  → TextInputLayoutEngine shapes content
  → NativeWindow::set_ime_cursor_area(shaped caret)

WindowEvent::KeyboardInput
  → NativeApp maps platform key to TextInputCommand
  → TextInput updates committed text / selection
  → rebuild and redraw

WindowEvent::Ime(Preedit)
  → NativeApp maps platform event to TextInputCompositionEvent
  → TextInput updates temporary composition
  → TextInputLayoutEngine projects committed + preedit text
  → InputBox paints preedit underline and caret

WindowEvent::Ime(Commit)
  → TextInput atomically inserts committed text
  → composition clears
  → rebuild, reposition candidate area, redraw

ApplicationHandler::about_to_wait
  → CaretBlinkController::advance
  → rebuild only when the visible phase changes
  → ControlFlow::WaitUntil(next blink deadline)
```

## 5. 关键取舍

| 选择 | 结论 | 原因 |
| --- | --- | --- |
| `TextInput` 直接处理 `WindowEvent` | ❌ | 造成 base → platform 反向依赖，无法纯测试 |
| `TextInput` 实现 `Component` 并拥有 chrome | ❌ | 混合编辑基座与具体 input-box presentation |
| 用字符数估算 caret | ❌ | 比例字体、fallback、emoji 和 BiDi 会产生错误候选框位置 |
| preedit 直接写入 committed text | ❌ | cancel/update 会破坏文本和 selection |
| `TextInput` base + `InputBox` component | ✅ | 编辑语义可复用，具体组件拥有自己的 chrome |
| 独立 layout engine + immutable snapshot | ✅ | shaping 几何可复用，InputBox 保持纯 scene composition |

## 6. Current limitations 与演进前提

当前 vertical slice 是 single-line composer，支持键盘插入、grapheme-safe 左右移动与删除、
Shift selection、Home/End、Select All、IME composition 和 caret blink。它尚未实现 mouse
caret placement、drag selection、clipboard、undo/redo、password/read-only variant、提交命令
和 accessibility。

增加 mouse selection 前，应让 hit testing 消费同一份 shaped layout，不能另建字符宽度估算。
增加多行编辑器前，应先定义 soft wrap、vertical cursor affinity、scroll ownership 和 IME
candidate area 的多行语义。如果未来独立 crate 也需要相同基座，再评估从 `zeta-ui` 抽取；
当前不复制模型。

## 7. 长期不变量

- platform adapter 只暴露窄 forwarding contract；
- authoritative editing state 不进入 renderer；
- render component 不执行产品命令；
- composition 与 committed text 保持可区分；
- cursor/selection geometry 与实际 shaping 使用同一文本语义；
- 产品领域状态不得下沉到 `zeta-ui` 或 `zeta-winit`。
