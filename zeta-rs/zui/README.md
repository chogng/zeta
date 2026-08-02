# `zui`

> 本 README 是 `zui` crate 的实现权威文档。跨 crate 的渲染所有权、后端替换边界和长期演进由
> [`docs/rendering-architecture.md`](../../docs/rendering-architecture.md) 维护；通用组件的实现由
> [`zeta-ui`](../ui/README.md) 维护。

`zui` 是后端无关的原生 UI 框架。它把声明式 `Element` 解析为一次 `ComputedElement`，用同一份
计算几何驱动组件绘制与检查快照，并把最终 `UiScene` 交给 renderer。它不提供 Button、ActionBar
等组件，也不接触窗口、输入路由、产品状态或具体 GPU API。

## 1. 边界与依赖方向

| 能力 | Owner | 状态 |
| --- | --- | --- |
| Element row/column、fixed/fill、padding、gap 与 radius | `zui` | ✅ |
| `ComputedElement` 与每帧 `InspectionFrame` | `zui` | ✅ |
| geometry、paint/image/icon/text scene primitive | `zui` | ✅ |
| Scene 稳定前缀 checkpoint 与 volatile fragment 原地替换 | `zui` | ✅；host 决定 retained boundary |
| Split/Grid、text shaping 与单行 text input 基座 | `zui` | ✅ |
| Button、ActionBar、TabList、ContextView 等组件 | `zeta-ui` | 委托 |
| GPU execution 与 surface lifecycle | `zeta-renderer` / backend crate | 委托 |
| pointer、focus、command 与 accessibility | `zeta-ui-dispatch` / product host | 委托 |

允许的依赖方向是 `zeta-ui → zui`、`zeta-ui-dispatch → zui`、`zeta-renderer → zui` 和
`backend → zeta-renderer + zui`。`zui` 不得依赖这些上层 crate。出现 `wgpu::*`、产品 identity、
组件样式或 host reducer，意味着 ownership 已经漂移。

## 2. 文件与接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `Component` | public | 要求组件声明 `ComponentElement`，并允许绘制消费同一次 computed geometry |
| `Element` / `ElementStyle` | public | 保存 authored direction、length、padding、gap、radius 和 child tree |
| `ComponentElement::compute` | public | 在 host bounds 内解析 immutable `ComputedElement` |
| `compute_element` / `resolved_padding` / `child_bounds` | private | 分配 fixed/fill 主轴空间、裁剪 box 并生成准确 gap regions |
| `UiScene::draw_component` | public | compute 一次、自动注册检查节点，再调用 `paint_element` |
| `UiScene::with_element` | public | 让 content closure 与 overlay 进入相同 compute/inspection 管线 |
| `UiScene::with_current_layer_element` / `with_inspection_node` | private | 绑定 scene layer、inspection parent 与声明源码位置 |
| `UiScene::checkpoint` / `restore` | public | 保留 scene primitive/layer/inspection 稳定前缀，并复用分配原地替换后续 fragment |
| `FrameScheduler::request` / `take` | public | 合并平台帧之间的失效请求，并由 host 在一次 redraw 中消费最高级别的工作 |
| `InspectionFrame::register` | crate-private | 建立单帧 identity、parent、layer 和命中顺序 |
| `scene::batching::batches` | private | 保留跨 primitive 的真实插入顺序并合并连续同类 range |
| `font::new_font_system` / `font::mapping` | private | 固定 layout 与 renderer 共享的 locale/fallback/font mapping policy |
| `renderer_support` | public backend bridge | 只向 renderer adapter 暴露共享 shaping policy |

## 3. 执行路径与不变量

```text
Component::element
  → ComponentElement::compute
  → compute_element
  → ComputedElement::inspection_node
  → InspectionFrame::register
  → Component::paint_element(same ComputedElement)
  → UiScene primitives
  → SceneBatch
  → zeta-renderer::Renderer
```

`Element` tree 每帧解析，不持有跨帧 identity。`InspectionNodeId` 只能在当前 `UiScene` 生命周期内使用；
选中状态如果要跨帧保存，必须由 host 建立自己的稳定 identity。Padding 色块使用 clamp 后的 resolved
值，Inspector 同时可读取原始 `ElementStyle`。Gap 由 layout 生成实际矩形，不允许 Inspector 猜测。

`FrameScheduler` 只合并 `Render < Fragment < Rebuild` 帧请求，不调用窗口 API，也不决定产品状态如何变化。
`Fragment` 表示由 host 划定的任意 presentation fragment，不表示 Overlay、Picker 或其他产品拓扑。Host 在收到
`FrameSchedule::RequestFrame` 时唤醒平台，在 redraw 开始时通过 `take` 消费工作；同步完成了等价重建时
必须调用 `clear`，避免下一帧重复执行。

`SceneCheckpoint` 是当前 retained presentation 的最小边界：它记录 primitive、composition layer 与
inspection prefix，`restore` 后 host 可以只重建 volatile fragment，并复用 Scene Vec capacity。它不提供
跨 scene identity，也不自动判断 dirty subtree；checkpoint 必须由创建它的同一 `UiScene` 消费。

Overlay Element 会创建高于当前 layer 的 scene layer，并在闭包返回后恢复调用者的 layer 与 clip。
panic recovery 不是当前 contract；组件 paint panic 会中断本帧构建。

## 4. 集成义务

- 组件调用者使用 `UiScene::draw_component`，不能直接调用 `Component::paint`。
- 拥有 content closure 的 surface 使用 `UiScene::with_element`。
- Renderer 只消费 `UiScene`，不得获得 Component、interaction 或 accessibility frame。
- `zeta-ui` 可以兼容 re-export zui API，但 renderer 和 dispatch 必须直接依赖 `zui`，避免基础协议由
  组件 crate 反向拥有。

## 5. 测试与修改影响

运行 `cargo test -p zui` 验证 Element layout、inspection、scene batching、font/text layout、text input、
Split/Grid 和 primitive contract。修改以下边界时还需同步验证：

- scene primitive 或 batch ordering：`zeta-renderer` 与全部 backend；
- font/text contract：`zeta-wgpu`、`zeta-ui`、`zeta-editor`、`zeta-markdown`；
- Element/Component contract：`zeta-ui`、`zeta-keybinding`、`zeta-editor` 与 native 架构审计；
- geometry/layout contract：`zeta-ui-dispatch` 和 native root/split/grid tests。

## 6. 当前限制与扩展点

当前具备帧级失效合并和 host 划定的 Scene prefix checkpoint，但没有 retained mount lifecycle、自动子树级
dirty propagation、跨帧 layout cache、style cascade、运行时样式编辑、path primitive 或远程
Inspector protocol。后续只有在真实消费者要求 retained identity/invalidation 时，才应扩展 Element
lifecycle；GPU backend 变化不应改变 `zui` public contract。
