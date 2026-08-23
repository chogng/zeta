# `zui`

> 本 README 是 `zui` crate 的实现权威文档。跨 crate 的渲染所有权、后端替换边界和长期演进由
> [`docs/rendering-architecture.md`](../docs/rendering-architecture.md) 维护；通用组件的实现由
> [`zeta-ui`](../ui/README.md) 维护。Native split host 的渐进式弃用边界由
> [`docs/native-deprecation-plan.md`](../docs/native-deprecation-plan.md) 维护。

`zui` 是后端无关的原生 UI 框架。它把声明式 `Element` 解析为一次 `ComputedElement`，用同一份
计算几何驱动组件绘制与检查快照，并把最终 `UiScene` 交给 renderer。它还提供后端无关的交互帧、
稳定控件身份和 pointer/focus 分发语义。它不提供 Button、ActionBar 等组件，也不调用窗口 API、
映射产品命令或保存产品状态。

## 1. 边界与依赖方向

| 能力 | Owner | 状态 |
| --- | --- | --- |
| Element row/column、fixed/fill、padding、gap 与 radius | `zui` | ✅ |
| `ComputedElement` 与每帧 `InspectionFrame` | `zui` | ✅ |
| geometry、paint/image/icon/text scene primitive | `zui` | ✅ |
| Renderer-independent icon asset contract | `zeta-icon`（由 `zui` facade re-export） | 委托 |
| Scene 稳定前缀 checkpoint 与 volatile fragment 原地替换 | `zui` | ✅；host 决定 retained boundary |
| Retained fragment lifecycle、animation cleanup 与 deadline report | `zui::RetainedRuntime` / `zui::RetainedFragmentRegistry` | ✅；host 应用 scene/interaction cleanup |
| Backend-neutral scalar animation、stable property key、deadline 与 advance contract | `zui::AnimationRegistry` / `zui::ScalarAnimation` | ✅；host 提供时间、调度唤醒和重绘 |
| Split/Grid、text shaping 与单行 text input 基座 | `zui` | ✅ |
| Button、ActionBar、TabList、ContextView 等组件 | `zeta-ui` | 委托 |
| GPU execution 与 surface lifecycle | `zeta-renderer` / backend crate | 委托 |
| 命中、pointer capture、focus、键盘导航、cursor 与 accessibility snapshot | `zui` | ✅ |
| 平台事件转换、cursor/accessibility 发布与产品命令映射 | product host / platform adapter | 委托 |

允许的依赖方向是 `zeta-ui → zui → zeta-icon`、`zeta-renderer → zui` 和
`backend → zeta-renderer + zui`。`zui` 不得依赖这些上层 crate。出现 `wgpu::*`、产品 identity、
组件样式或 host reducer，意味着 ownership 已经漂移。

### 内部分层

`lib.rs` 只是显式公共 facade，不承载实现。内部依赖固定为：

| 层 | 物理目录 | 可以依赖 | 明确不拥有 |
| --- | --- | --- | --- |
| 基础值 | `foundation/` | 无 | layout、scene、interaction、平台类型 |
| 几何算法 | `layout/` | `foundation` | Element、paint、产品 Pane 状态 |
| 文本内核 | `text/` | `foundation`、自身子模块 | scene、组件 chrome、IME 平台 lifecycle |
| 呈现组合 | `presentation/` | `foundation`、`text` | 跨帧 focus/capture、窗口事件、产品 reducer |
| 框架运行时 | `runtime/` | `foundation` | scene、component、text、产品 command |
| Renderer bridge | `renderer_support.rs` | `text` | scene mutation、GPU backend、平台 surface |

`presentation` 与 `runtime` 是并列层：product host 负责把一帧 `UiScene`、`InteractionFrame` 和
`FrameScheduler` 组合起来，任何一侧都不得反向获得另一侧。内部实现必须通过 canonical layer path
引用依赖，不能经由 crate root re-export 绕过层次。生产模块上限为 500 行；新增职责应进入所属层的
新私有模块。

## 2. 文件与接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `Component` | public | 要求组件声明 `ComponentElement`，并允许绘制消费同一次 computed geometry |
| `Element` / `ElementStyle` | public | 保存 authored direction、length、padding、gap、radius 和 child tree |
| `ComponentElement::compute` | public | 在 host bounds 内解析 immutable `ComputedElement` |
| `compute_element` / `resolved_padding` / `child_bounds` | private | 分配 fixed/fill 主轴空间、裁剪 box 并生成准确 gap regions |
| `UiScene::draw_component` | public | compute 一次、自动注册检查节点，再调用 `paint_element`；适用于不需要交互 sink 的 scene-only 组合 |
| `ComponentContext` / `UiFrame` | public | 在同一 frame 中携带 inspection parent、interaction sink 与显式时钟，组合子组件并声明 modal scope |
| `UiFrame::draw_component` / `with_context` / `with_element` / `with_clip` | public | canonical frame composition；低层 host paint 也必须留在 frame 的 scoped closure 内 |
| `ComponentContext::with_component` | public | 在一个组件根下交错自定义 paint 与子组件，同时保持 scene inspection parent、interaction parent 和 animation binding |
| `UiFrame::with_animation_bindings` / `draw_component_with_animation_bindings` | public | 将 retained `AnimationBinding` scoped 到一帧组件组合；组件不直接依赖 runtime registry |
| `presentation::inspection::node_for_element` | crate-private | 单向把 `ComputedElement` 投影为 inspection metadata；Element 层不反向依赖 inspection |
| `presentation::component` 中的 `impl UiScene::draw_component` | public binding | 把 Component 接入 scene；scene 核心不依赖 Component trait |
| `UiScene::with_element` | public | 让 content closure 与 overlay 进入相同 compute/inspection 管线 |
| `UiScene::with_current_layer_element` / `with_inspection_node` | private | 绑定 scene layer、inspection parent 与声明源码位置 |
| `UiScene::checkpoint` / `restore` | public | 保留 scene primitive/layer/inspection 稳定前缀，并复用分配原地替换后续 fragment |
| `UiScene::with_fragment` / `replace_fragment` / `remove_fragment` | public | 以稳定 `ElementId` 管理 terminal retained fragment，并在移除时恢复 scene/inspection prefix |
| `FrameScheduler::request` / `take` | public | 合并平台帧之间的失效请求，并由 host 在一次 redraw 中消费最高级别的工作 |
| `FrameDeadlineSet::include` / `next_deadline` | public | 聚合组件、动画和异步轮询的最早单调 deadline；不接触平台 event loop |
| `ScalarAnimation::transition_to` / `advance` | public | 插值一个后端无关的标量值，支持 retarget、easing、deadline 与 snap/cancel |
| `AnimationRegistry::transition_to` / `advance` | public | 按 `ElementId + AnimationProperty` 保留跨帧 track，聚合 changed keys、fragment IDs、失效等级与 deadline |
| `RetainedFragmentRegistry::mount` / `begin_exit` / `advance` | public | 保留稳定 fragment lifecycle，取消退出、按显式时钟生成到期 cleanup IDs，并投影局部失效 |
| `RetainedRuntime::mount` / `begin_exit` / `advance` | public | 聚合 fragment lifecycle 与 animation tracks；fragment 到期或 unmount 时自动清理其 animation tracks |
| `InteractionFrame::register` / `checkpoint` / `restore` | public | 记录一帧的绘制顺序、交互节点与 modal scope，并与 retained scene prefix 对齐 |
| `InteractionSink::register` / `set_modal_root` | public | 为 presentation 组合提供后端无关的交互节点与 modal boundary；runtime frame 承担实际保留和分派 |
| `UiDispatch` | public | 跨 frame 保存 hover、press/capture、focus 与窗口激活状态，只产生无效请求和 `UiIntent` |
| `UiNode` / `ElementId` / `AccessibilityNode` | public | 分别表达当前帧控件语义（含交互失效等级）、跨帧稳定 identity 与不可变 accessibility snapshot；不分配产品 identity 或发布平台 API |
| `InspectionFrame::register` | crate-private | 建立单帧 identity、parent、layer 和命中顺序 |
| `scene::batching::batches` | private | 保留跨 primitive 的真实插入顺序并合并连续同类 range |
| `font::new_font_system` / `font::mapping` | private | 固定 layout 与 renderer 共享的 locale/fallback/font mapping policy |
| `renderer_support` | public backend bridge | 只向 renderer adapter 暴露共享 shaping policy |

## 3. 执行路径与不变量

```text
UiScene::draw_component
  → Component::element
  → ComponentElement::compute
  → compute_element
  → presentation::inspection::node_for_element
  → InspectionFrame::register
  → Component::paint_element(same ComputedElement)
  → UiScene primitives
  → SceneBatch
  → zeta-renderer::Renderer
```

新宿主的 canonical 入口是 `UiFrame::draw_component`：它让 scene、inspection、interaction 和 frame clock
由一个 frame owner 进入同一次组合。需要自定义低层 paint 与子组件交错时，使用
`ComponentContext::with_component`；需要无交互的低层 element closure 时才使用 `with_element`，让
`ComponentContext` 继续拥有同一个 frame。
旧的 `UiScene::draw_component_with_interaction`、`UiFrame::parts_mut` 和 `UiFrame::into_parts` 已删除，
不得重新引入平行的 scene/interaction 输出接口。

`Element` tree 每帧解析，不持有跨帧 identity。`InspectionNodeId` 只能在当前 `UiScene` 生命周期内使用；
选中状态如果要跨帧保存，必须由 host 建立自己的稳定 identity。Padding 色块使用 clamp 后的 resolved
值，Inspector 同时可读取原始 `ElementStyle`。Gap 由 layout 生成实际矩形，不允许 Inspector 猜测。
`element` 不引用 `inspection`，`scene` 不引用 `Component`；这两个单向绑定分别由
`presentation::inspection` 和 `presentation::component` 承担，禁止重新引入双向模块依赖。

`FrameScheduler` 只合并 `Render < Fragment < Rebuild` 帧请求，不调用窗口 API，也不决定产品状态如何变化。
`Fragment` 表示由 host 划定的任意 presentation fragment，不表示 Overlay、Picker 或其他产品拓扑。Host 在收到
`FrameSchedule::RequestFrame` 时唤醒平台，在 redraw 开始时通过 `take` 消费工作；同步完成了等价重建时
必须调用 `clear`，避免下一帧重复执行。

`SceneCheckpoint` 是当前 retained presentation 的最小边界：它记录 primitive、composition layer 与
inspection prefix，`restore` 后 host 可以只重建 volatile fragment，并复用 Scene Vec capacity。它不提供
跨 scene identity，也不自动判断 dirty subtree；checkpoint 必须由创建它的同一 `UiScene` 消费。

`RetainedRuntime` 是跨帧 retained state 的通用 owner；其内部的 `RetainedFragmentRegistry` 管理 fragment
lifecycle，`AnimationRegistry` 管理属性 track。Host 在当前组合出现稳定
identity 时调用 `mount`；重复调用表示 update，不会重置 retained state。组件离开当前组合时调用
`begin_exit`，退出期间不能重新注册 interaction 或 inspection 节点；`advance` 到达 deadline 后返回
cleanup IDs，并自动移除这些 identity 的 animation tracks。Host 必须用这些 ID 调用
`UiScene::remove_fragment`，并恢复配对的 `InteractionFrameCheckpoint`；如果 fragment 不是 terminal，则
退回 `FrameInvalidation::Rebuild`，不能静默破坏后续 paint order。

Overlay Element 会创建高于当前 layer 的 scene layer，并在闭包返回后恢复调用者的 layer 与 clip。
panic recovery 不是当前 contract；组件 paint panic 会中断本帧构建。

交互沿同一份 layout bounds 运行：host 在 scene 构建时注册 `UiNode`，平台事件进入 `UiDispatch`，
其返回的 `UiIntent` 再由 host 映射为产品状态转换。`zui` 不接受 callback registry，也不执行
window drag、菜单、Session 或 editor command。

## 4. 集成义务

- 组件调用者使用 `UiScene::draw_component`，不能直接调用 `Component::paint`。
- 拥有自定义 paint 与子组件的 surface 使用 `ComponentContext::with_component`；无交互 scene-only
  closure 才使用 `UiScene::with_element`。
- Renderer 只消费 `UiScene`，不得获得 Component、interaction 或 accessibility frame。
- 使用同一份绘制 bounds 注册 `UiNode`，不能另行估算命中区域；动态对象在仍表示同一对象时保持
  `ElementId`。
- retained fragment 的 mount/update/unmount 必须由 `RetainedRuntime` 记录；退出 fragment
  不得继续出现在当前 interaction 或 inspector snapshot 中。
- 处理 `RetainedRuntimeAdvanceReport::fragment().removed_ids` 时，scene 和 interaction checkpoint 必须
  在同一 cleanup 路径收敛；动画 track 由 runtime 自动清理，不能只删 paint 而留下可交互或可检查的 ghost node。
- product host 将 `UiIntent` 映射为 command，并由 platform adapter 发布 cursor 与 accessibility
  snapshot；`zui` 不建立第二套业务 reducer 或平台 adapter。
- `zeta-ui` 可以兼容 re-export `zui` API，但 renderer 必须直接依赖 `zui`，避免基础协议由组件 crate
  反向拥有。

## 5. 测试与修改影响

运行 `cargo test -p zui` 验证 Element layout、inspection、scene batching、font/text layout、text input、
Split/Grid、interaction 与 primitive contract。修改以下边界时还需同步验证：

- `architecture_tests` 验证物理分层、canonical dependency path、500 行模块上限、显式 root export，
  以及平台、GPU、组件和产品依赖禁令；

- scene primitive 或 batch ordering：`zeta-renderer` 与全部 backend；
- font/text contract：`zeta-wgpu`、`zeta-ui`、`zeta-editor`、`zeta-markdown`；
- Element/Component contract：`zeta-ui`、`zeterm-keybinding-ui`、`zeta-editor` 与 native 架构审计；
- geometry/layout/interaction contract：native root/split/grid tests。

## 6. 当前限制与扩展点

当前具备帧级失效合并、host 划定的 Scene/interaction prefix checkpoint、terminal fragment 移除、
`RetainedRuntime`、`RetainedFragmentRegistry`、`AnimationBinding` 和通用 pointer/focus 语义，但没有自动子树级
dirty propagation、跨帧 layout cache、style cascade、运行时样式编辑、path primitive、disabled/live-region/
text-selection accessibility semantics 或平台 accessibility adapter。产品级 exit retention 仍需各 fragment
明确保留内容、退出目标和 reduced-motion policy；GPU backend 变化不应改变 `zui` public contract。
