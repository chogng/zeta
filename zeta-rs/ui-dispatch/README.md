# `zeta-ui-dispatch`

> zeterm 的产品结构、Session TabList 与长期交互边界见
> [`docs/native-terminal-ui.md`](../../docs/native-terminal-ui.md)。本 README 只拥有跨 native
> 组件 UI 分发基座的当前实现契约、接入义务和扩展信号。

`zeta-ui-dispatch` 在 immutable presentation frame 与窗口事件之间提供稳定控件身份。它统一
命中测试、hover path、press/capture、focus、键盘导航、cursor、activation intent 和
accessibility semantics，但不保存 Session、文件树、Tab、Chat、Editor 或任何业务模型。

## 所有权

| 能力 | Owner | 状态 |
| --- | --- | --- |
| 跨 frame 稳定控件身份 | `ElementId` | ✅ |
| 当前 frame 节点、绘制顺序命中与 focus order | `InteractionFrame` / `UiNode` | ✅ |
| hover、press、pointer capture 与 focus identity | `UiDispatch` | ✅ |
| Tab 顺序与同组水平/垂直导航 | `UiDispatch` / `NavigationGroupId` | ✅ |
| activation/window-drag intent | `UiIntent` / `NodeAction` | ✅ |
| Default/Text/Pointer/ResizeHorizontal cursor projection | `CursorFeedback` | ✅ |
| role、label、value、bounds、focus 与 selection snapshot | `AccessibilityNode` | ✅ |
| 布局、绘制和组件内部 geometry | `zeta-ui` / product component | ❌ |
| Session、filesystem、document、chat 或 command state | 对应 product owner | ❌ |
| VoiceOver、Narrator、Orca 等平台发布 | 后续 window adapter | 尚未完成 |

依赖方向只有：

```text
product host → zeta-ui-dispatch → zeta-ui geometry
             → zeta-ui components/scene
```

本 crate 不能依赖 `zeta-native`、terminal、workspace、session 或 editor domain。出现这些依赖
说明通用分发边界已经漂移。

## 接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `ElementId::scoped` | public | 由 consumer scope 与 local identity 构造稳定身份；不分配业务 ID |
| `UiNode` | public | 绑定一帧中的 bounds、parent、cursor、focus policy、action、navigation group 与 semantics |
| `CursorFeedback::ResizeHorizontal` / `AccessibilityRole::Separator` | public | 投影 Sash 的横向尺寸调整 cursor 与 separator 语义；不拥有 drag state 或 Pane 尺寸 |
| `AccessibilityRole::Menu` / `MenuItem` | public | 投影菜单父子语义；不拥有打开、关闭、焦点恢复或产品 command |
| `InteractionFrame::register` | public | 按 scene/paint 顺序记录节点；同一 identity 每帧只能注册一次 |
| `InteractionFrame::set_modal_root` | public | 把 pointer target 与 focus order 限定在一个已注册子树；下层节点在该 frame 内保持 inert |
| `InteractionFrame::target_at` | public | 逆序选择最上层命中节点 |
| `InteractionFrame::ancestry` | public | 从命中节点投影到父节点 hover path；modal frame 在 modal root 截断 |
| `InteractionFrame::focus_order` | public | 按注册顺序返回 active scope 内的 `TabStop` |
| `InteractionFrame::accessibility_nodes` | public | 把 frame semantics 与当前 focus 合并为 immutable snapshot |
| `UiDispatch::pointer_moved` | public | 更新 hover path，只在视觉状态改变时请求 paint |
| `UiDispatch::press_primary` / `release_primary` | public | 建立 pointer capture，并只在 release 回到原节点时产生 activation |
| `UiDispatch::reconcile_focus` | public | frame rebuild 后保留仍有效的 focus，否则选择 preferred/首个 tab stop |
| `UiDispatch::focus_in_order` | public | 实现 Tab/Shift+Tab 环形遍历 |
| `UiDispatch::focus_within_group` | public | 实现 Toolbar/TabList 等组件内同轴导航 |
| `UiDispatch::activate_focused` | public | 把 Enter/Space 转为 focused `UiIntent` |
| `DispatchOutcome` | public | 只返回 paint invalidation 与 intent，不执行 product command |

实际调用路径：

```text
product layout + component geometry
  → InteractionFrame::register(UiNode)
  → platform pointer/key event
  → UiDispatch
  → DispatchOutcome
      ├─ Paint → product rebuilds scene + InteractionFrame
      └─ UiIntent → product maps ElementId to command/state transition
  → InteractionFrame::accessibility_nodes
  → future platform accessibility adapter
```

## 接入义务

- component owner 必须使用绘制采用的同一份 bounds 注册节点，不能另行估算 hit region；
- 动态 file/tree row、tab、chat surface 或 editor view 在仍表示同一对象时必须保持
  `ElementId`；
- parent identity 表达语义和 hover ancestry，不转移业务 ownership；
- Button、Tab 等 runnable control 使用 `FocusBehavior::TabStop` 与
  `NodeAction::Activate`；
- Menu owner 注册 `Menu` 父节点与 `MenuItem` action；outside click、Escape、焦点恢复和
  command mapping 仍由 product host 负责；打开期间用 `set_modal_root` 隔离下层 pointer 与
  focus；
- Sash 使用 `zeta-ui::Sash::interaction_bounds` 注册 `Separator` 与对应 resize cursor；当前
  product host 单独保存 drag snapshot，不把 Pane 尺寸放进 `UiDispatch`；
- Toolbar、TabList 等使用稳定 `NavigationGroupId`，并明确水平或垂直轴；
- product host 负责把 `UiIntent` 映射到 command，crate 不接收 callback registry；
- accessibility adapter 必须发布 `AccessibilityNode`，不能建立第二套 focus 或 selection。

`InteractionFrame` 是一帧数据，允许每次 layout 后重建；`UiDispatch` 才跨 frame 保存短期交互
状态。窗口失焦会清除 hover、press 和 capture，但保留 focus identity，以便重新激活时恢复。
不存在的 focused element 会在下一次 `reconcile_focus` 被替换。

## 测试与当前限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-ui-dispatch
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-ui-dispatch --all-targets -- -D warnings
```

测试覆盖反向命中、父子 hover、pointer capture、Tab 顺序、同组导航、键盘 activation、窗口
blur/focus 和 accessibility snapshot。

当前没有 disabled、expanded、checked、live region、text selection range、separator
orientation/value-range action 或 accessibility action adapter；Sash 尚未接入键盘 resize。
这些状态只有在真实组件和平台 adapter 需要时才扩展。当前也没有 retained widget tree、
layout、paint、command registry 或业务 reducer。把这些能力加入本 crate 会改变 ownership，
需要同步评审本 README 与 `docs/native-terminal-ui.md`。
