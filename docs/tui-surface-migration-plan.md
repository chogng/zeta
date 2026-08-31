# `zeta code` 终端界面、占高区域与临时层迁移计划

> 状态：Accepted migration plan。
> 范围：`zeta-code/tui` 的顶层界面切换、占高交互区域、不占高临时层、聊天输入补全、输入路由、绘制和命中。
> 架构所有权：长期不变量与最终当前状态归 [`tui.md`](tui.md)；当前实现接口归 [`zeta-code/tui/README.md`](../zeta-code/tui/README.md)；本文只拥有本次迁移的设计决定、执行顺序和验收清单。

## 快速理解

TUI 只保留三种空间语义：终端界面决定整屏内容，占高区域参与当前界面的行数分配，临时层覆盖在当前帧上且不改变任何区域高度。聊天输入补全属于 `ChatInput`，虽然按临时层绘制，但不进入应用级临时层状态。

| 用户看到什么 | 是否改变当前界面 | 是否占用布局高度 | 状态 owner | 典型内容 |
| --- | --- | --- | --- | --- |
| 终端界面 | 是 | 使用整个终端 | 对应 feature，App 只协调切换 | Session 对话、Session Manager |
| 占高区域 | 否 | 是 | 对应 feature；通用 component 只负责交互和绘制 | Help、Config、Theme、Skills、Queue |
| 应用级临时层 | 否 | 否 | App 管唯一活动项；内容状态归对应 component/feature | Status、Session preview、正文详情、Queue 详情 |
| 输入补全 | 否 | 否 | `ChatInput` | `/`、`@`、`$` 候选 |

最终输入顺序、生命周期、迁移映射和验收要求分别见第 3、4、6、8 节。

## 1. 目标与非目标

本次迁移解决四个问题：

- 顶层 Session Manager 和 Session 对话不再使用含义模糊的 root 术语，直接表达当前终端界面。
- `PaneStack` 不再同时承担内容类型、页面栈、输入路由、高度、提示和 feature action 绑定。
- `QuickView` 不再作为独立的第四种界面类型；它收敛为应用级只读临时层。
- `/`、`@`、`$` 补全继续完整归 `ChatInput`，App 和占高区域不能复制它的 token、selection 或 dismiss 状态。

本次迁移不改变 App Server contract、Session/Thread 产品事实、Turn 请求语义、补全 catalog 内容、配置持久化格式或正文数据模型。

## 2. 最终责任边界

| Owner | 拥有 | 不拥有 |
| --- | --- | --- |
| `features/sessions` | 当前 `TerminalScreen`、Manager 选择状态、每个 Session 最近查看的 Thread | 应用级临时层、聊天输入补全、其他 feature 的占高区域 |
| `app/active_region.rs` | 当前是否存在一个占高区域、把输入和绘制委托给准确 feature、区域打开/替换/关闭协调 | feature action map、列表选择细节、业务请求、页面内部返回关系 |
| `components/region.rs` | 占高区域的标题边界、正文几何、通用提示、列表/文本/按键录制的机械绘制与命中 | 全局生命周期、RPC、feature action、页面栈 |
| `features/*/region.rs` | 本 feature 的页面状态、选项到 typed intent 的绑定、多页面返回关系、刷新后的状态替换 | 终端界面切换、全局焦点、终端几何、直接执行副作用 |
| `app/active_overlay.rs` | 至多一个应用级临时层、打开/替换/关闭和输入优先级 | 补全状态、业务事实、正文绘制细节 |
| `components/overlay.rs` | 只读详情临时层的大小、滚动、背景、提示和绘制 | RPC、feature action、全局生命周期、任意可交互页面 |
| `components/chat_input/` | draft、cursor、attachments、token、唯一 completion、selection、应用和 dismiss | 应用级临时层、占高区域、终端界面切换 |
| `app/frame.rs` | 当前终端界面、占高区域和当前帧临时层的绘制顺序 | 推进交互状态、发请求、决定 feature action |
| `app/screen_layout.rs` | 只给占高内容分配行数 | 临时层高度、补全状态、feature 生命周期 |

长期判断规则是：状态跟随谁的生命周期，就由谁保存；容器只保存完成布局和输入委托所必需的状态。

## 3. 状态模型

### 3.1 终端界面

`features/sessions` 使用可穷举的 `TerminalScreen`：

```rust
enum TerminalScreen {
    Manager,
    Session(SessionId),
}
```

切换终端界面时，App 关闭活动占高区域和应用级临时层，清除它们的 pointer 状态，并保留各 Thread 自己拥有的聊天草稿、Queue、scroll 和正文缓存。

### 3.2 占高区域

App 同时最多协调一个菜单类占高区域。活动项使用可穷举 enum，variant 保存 feature 自己定义的 region state；不再使用 `PaneId → PaneActions` 平行表，也不使用跨 feature 的通用页面栈。

简单 feature 直接拥有一个带 typed action 的列表区域。Config、Theme、Keymap 等多页面流程在自己的 feature 内保存当前页面和返回关系。关闭子页面时只修改该 feature 的 region state；关闭最外层页面时才清空 App 的活动区域。

Approval、Query、Goal、Plan、Queue 摘要和 Subagent 区域已经有独立 feature owner，继续作为当前终端界面的占高区域参与 `screen_layout`，不塞入菜单活动区域。

### 3.3 应用级临时层

App 同时最多保存一个应用级只读临时层。当前内容限定为详情展示，不允许把 ListSelection、TextPrompt、KeyCapture 或 feature 页面流塞进通用临时层。

打开新的应用级临时层会替换旧临时层。Esc 关闭；滚动键只改变临时层自己的 scroll。其他普通输入不会穿透到底层界面。

### 3.4 输入补全

`ChatInput` 继续保存唯一 completion。App 不创建第二份 completion enum，也不把 completion 放进应用级临时层字段。每帧按以下规则选择可见临时内容：

1. 有应用级临时层时，只绘制该临时层。
2. 没有应用级临时层、没有占高菜单区域且 ChatInput 有 completion 时，绘制 completion。
3. 其他情况不绘制临时内容。

## 4. 生命周期与输入顺序

### 4.1 生命周期

| 事件 | 占高区域 | 应用级临时层 | ChatInput 与 completion |
| --- | --- | --- | --- |
| 打开占高区域 | 替换当前区域 | 关闭 | 保留 draft；暂停绘制和处理 completion |
| 关闭占高区域 | 清空或返回 feature 内上一页 | 不变 | 恢复 ChatInput；由当前 token 决定 completion |
| 打开临时层 | 保留在下方 | 替换当前临时层 | 保留 draft 和 completion state，但不绘制 completion |
| 关闭临时层 | 不变 | 清空 | 恢复底层焦点和 completion 绘制 |
| 切换终端界面 | 关闭 | 关闭 | 切到目标 Thread 自己的 ChatInput；Manager 使用自己的输入语义 |
| Approval/Query 到达 | 保留菜单区域状态但请求交互取得当前输入位置 | 关闭 | 保留 draft；请求结束后恢复 |

### 4.2 输入顺序

输入传播固定为：

```text
终端级捕获
  → 应用级临时层
  → Approval / Query 等请求交互
  → 活动占高区域
  → 当前终端界面
  → ChatInput（completion 先于文本编辑）
  → 应用级快捷键与退出处理
```

仅 Chord prefix、终端恢复和字符框选等真正的终端级行为可以位于临时层之前。Esc、Ctrl-C、鼠标和 slash command 都必须复用这条优先级，不能分别维护另一套判断。

## 5. 布局、绘制与命中

`app/screen_layout.rs` 只接收各占高区域的 desired rows。菜单占高区域与普通 ChatInput 共用交互位置：存在菜单区域时绘制菜单区域并隐藏 ChatInput；不存在时绘制 ChatInput。菜单区域的 desired rows 不包含被隐藏 ChatInput 的高度。

`app/frame.rs` 的顺序固定为：

1. 清理背景并绘制当前 `TerminalScreen`。
2. 绘制 Goal、Plan、Queue、Query、Approval、交互位置、状态栏和 Subagent 等占高区域。
3. 绘制当前帧唯一临时内容：应用级详情临时层或 ChatInput completion。
4. 最后绘制字符框选。

命中测试使用相反的视觉优先级。应用级临时层存在时不返回底层 pointer target；菜单区域存在时不返回 ChatInput 或 completion target；completion 只命中自身可见 viewport。

## 6. 旧实现到最终 owner 的映射

| 旧实现 | 最终归属 | 处理 |
| --- | --- | --- |
| `features/sessions::RootTarget` | `features/sessions::TerminalScreen` | 重命名并把切换清理交给 App |
| `components/pane::PaneSpec` | `components/region::RegionSpec` | 只保留标题正文和附加提示的构造数据 |
| `components/pane::PaneStack` | `app::active_region::ActiveRegion` + feature region state | 删除通用栈；多页面返回关系下沉到对应 feature |
| `PaneId` + `App::pane_actions` | feature region 中的 typed action binding | 删除平行身份表和 App 中的业务 action 大 switch |
| `ChatComposer` 的 pane 路由 | `app::active_region` | `ChatComposer` 只协调 ChatInput 提交目标和 Steer |
| `components/quick_view.rs` | `components/overlay.rs` + `app::active_overlay.rs` | 限定为不占高只读详情临时层 |
| `App::quick_view` | `App::active_overlay` | 统一打开、替换、关闭和输入优先级 |
| `ChatComposerPaneView` | `RegionView` | 区域视图不再属于 composer |
| frame 内部直接叠加 completion 与 QuickView | 单一帧级临时内容选择 | 保证同帧最多显示一种临时内容 |

## 7. 测试策略

| 层级 | 必须固定的行为 |
| --- | --- |
| `components/region` | desired rows、标题/正文几何、列表命中、搜索和 Tab 命中、窄终端裁剪 |
| feature region | typed action、页面进入/返回、刷新替换、Esc 关闭边界、错误后保留当前页面 |
| `components/overlay` | 不改变布局、滚动边界、Esc 关闭、绘制高度限制 |
| `ChatInput` | `/`、`@`、`$` 同时最多一种、选择应用、Esc dismiss、鼠标命中 |
| App state | 输入优先级、界面切换清理、区域与临时层互斥规则、请求交互恢复、草稿保留 |
| frame/layout | 临时层不进入 desired rows、菜单区域替换 ChatInput、绘制和命中优先级 |
| stale-reference | 不再出现 `PaneStack`、`PaneId`、`PaneActions`、`QuickViewState`、`RootTarget` |

验证使用 `just test zeta-tui` 和 `just check zeta-tui`。只有定向验证通过且共享 workspace contract 确实受影响时才考虑完整 workspace 验证。

## 8. 执行 Checklist

### A. 行为基线与文档

- [x] 固定 slash completion、应用级详情临时层、菜单区域输入优先级和界面切换行为测试。
- [x] 写明三种空间语义、owner、生命周期、输入顺序、绘制顺序和迁移映射。
- [x] 在 `docs/tui.md` 和 crate README 中链接本计划，并改写为当前事实。

### B. 终端界面

- [x] 用 `TerminalScreen` 替换 `RootTarget`、`root`、`previous_root` 和 `next_root`。
- [x] 把界面切换集中到 App，切换时关闭活动区域和应用级临时层。
- [x] 保留每个 Thread 的 ChatInput、Queue、scroll 和正文缓存。

### C. 占高区域

- [x] 建立 `components/region.rs`，只提供区域构造数据、机械交互视图、绘制、desired rows 和命中。
- [x] 建立 `app/active_region.rs`，只协调一个活动区域并委托给 feature。
- [x] 把简单列表的 typed action 与 `ListSelectionState` 合并进对应 feature region。
- [x] 把 Config 的设置列表和 API key 输入作为同一个 feature flow。
- [x] 把 Keymap 的列表、action menu 和 key capture 作为同一个 feature flow。
- [x] 把 Theme 的内置列表和 Custom 列表作为同一个 feature flow。
- [x] 把 Queue 详情打开动作改为应用级临时层，不改变 Queue region。
- [x] 从 `ChatComposer` 删除区域状态、区域栈、区域绘制和区域 pointer routing。
- [x] 从 App 删除 `PaneId → PaneActions` 平行表和对应业务分派。

### D. 应用级临时层与补全

- [x] 用 `components/overlay.rs` 替换 `quick_view.rs`，内容只保留只读详情。
- [x] 建立 `app/active_overlay.rs`，统一打开、替换、Esc/滚动和关闭。
- [x] Status、Session preview、正文详情和 Queue 详情全部走同一应用级临时层入口。
- [x] 应用级临时层存在时阻止底层键盘和 pointer 输入。
- [x] completion 继续由 `ChatInput` 保存；App 不新增 completion state。
- [x] frame 在应用级临时层和 completion 之间只选择一个绘制。

### E. 布局与绘制

- [x] `screen_layout` 只计算占高区域，临时层不得进入 desired rows。
- [x] 菜单区域存在时替换 ChatInput，并只使用自己的 desired rows。
- [x] frame 按“终端界面 → 占高区域 → 临时内容 → 字符框选”绘制。
- [x] pointer hit test 按视觉顺序反向执行，不能命中被覆盖内容。

### F. 退场、文档与验证

- [x] 删除 `components/pane.rs`、`components/pane/`、`components/quick_view.rs` 和旧测试文件。
- [x] feature 文件、事件和方法中的 `Pane` 命名全部改为 `Region`。
- [x] 更新 `docs/tui.md`、`zeta-code/tui/README.md`、`zeta-code/tui/src/README.md` 和 slash command 文档。
- [x] `rg` 确认生产代码和当前事实文档没有旧类型或旧 owner 描述；本计划的迁移映射保留历史名称。
- [x] 运行 Rust 格式化。
- [x] 运行 `just test zeta-tui`（471 passed）。
- [x] 运行 `just check zeta-tui`。
- [x] 审查最终 diff，确认没有无关改动、双 owner 或兼容转发层。

## 9. 完成条件

只有同时满足以下条件才算完成：

- 代码中只剩终端界面、占高区域和不占高临时层三种空间语义。
- App 只保存当前界面、当前活动区域和当前应用级临时层，不保存 feature action 平行表。
- 多页面流程都能在 feature 内解释其返回关系。
- `ChatInput` 仍是 completion 的唯一状态 owner。
- 临时层不会改变 `screen_layout` 的任何高度输入。
- 所有旧类型和文档引用都已删除，定向测试和 crate check 通过。
