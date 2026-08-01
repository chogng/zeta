# UI 渲染架构

> 状态：当前实现。
> 本文拥有组件到图形后端之间的跨 crate 边界与替换规则。具体接口和实现细节分别见
> [`zeta-ui`](../zeta-rs/ui/README.md)、[`zeta-renderer`](../zeta-rs/renderer/README.md) 与
> [`zeta-wgpu`](../zeta-rs/wgpu/README.md)。

## 快速理解

组件只描述“画什么”，统一渲染接口决定“由哪个后端执行”，具体 GPU crate 才知道“如何提交”。
因此布局、组件检查器和产品交互不会因为 wgpu、Metal 或 Vulkan 实现替换而变化。

| 常见问题 | 当前行为 | 替换后端时是否改组件 |
| --- | --- | --- |
| 组件如何绘制矩形、文字、图标和图片？ | 向 `UiScene` 写入后端无关 primitive | ❌ |
| 谁执行一帧？ | product host 调用 `dyn Renderer` | ❌ |
| 谁接触 device、queue、shader 和 surface？ | 仅具体 backend crate | 不适用 |
| 当前后端是什么？ | `zeta-wgpu::WgpuRenderer` | 不适用 |
| 可以增加 raw Metal/Vulkan backend 吗？ | ✅，实现 `Renderer` 并替换 composition-root factory | ❌ |

```mermaid
flowchart LR
    C["Component / product composition"] --> S["zeta-ui::UiScene"]
    S --> R["zeta-renderer::Renderer"]
    R --> W["zeta-wgpu::WgpuRenderer"]
    R -. "可替换" .-> M["future Metal backend"]
    R -. "可替换" .-> V["future Vulkan backend"]
```

## 边界与所有权

| 层 | 决定什么 | 明确禁止 |
| --- | --- | --- |
| Component / product | 状态、布局、primitive 顺序、clip、overlay 与检查元数据 | GPU handle、shader、backend feature |
| `zeta-ui` | immutable scene contract、logical coordinates、paint semantics 与有序 `SceneBatch` | `wgpu::*` 或其他具体图形 API |
| `zeta-renderer` | target size、frame outcome、统一 backend error 与执行接口 | surface、window、pipeline、atlas |
| backend crate | physical conversion、batch execution、resource cache、shader、submit、present | 产品状态、组件 layout、输入路由 |
| product composition root | 选择并初始化 backend，实现类型擦除 | 把 backend 类型传播回组件或 scene |

布局检查器消费 `UiScene` 同步生成的 `InspectionFrame`。它位于 backend 之前，所以切换 GPU API
不会改变组件层级、尺寸、padding、radius 或 source location 的采集。

Native presentation 同时保存 `UiScene`、`InteractionFrame` 与 accessibility projection，但只把
`UiScene` 交给 `Renderer`。命中、焦点、cursor、command dispatch 和 accessibility 不属于 GPU
协议，也不会因更换 backend 而重新实现。

## 当前执行流程

1. Component 通过 `UiScene::draw_component` 组合子组件并发出 primitive。
2. `UiScene` 按 composition layer 与 primitive 插入顺序产生连续的 `SceneBatch`；不同 primitive
   类型之间的覆盖顺序不会被 backend 重排。
3. Native host 保存 scene、inspection、interaction 与 accessibility frame，不获得任何 GPU 对象。
4. Host 只把 scene 传给 `Box<dyn Renderer>::render_scene`。
5. 当前 `renderer_backend::create` 选择 `WgpuRenderer`；只有该 adapter 知道具体类型。
6. `zeta-wgpu` 把 batch 对应的 logical primitive range 转为 physical instances/glyph buffers，按
   scene 顺序切换 pipeline、提交并 present。

当前 wgpu 在 macOS、Linux 和 Windows 上本身可以选择 Metal、Vulkan 或 DX12 backend；本文边界
解决的是更进一步的实现替换，例如绕过 wgpu 编写专用 Metal/Vulkan renderer。

## 长期不变量

- `zeta-ui` 不直接依赖或引用任何具体 GPU API；
- Component 只能产生 scene primitive，不接受 backend context；
- backend 不重新解释产品布局、组件身份或交互状态；
- primitive 的 back-to-front 顺序由 scene contract 决定，backend 不得按自身 pipeline 偏好重排；
- interaction/accessibility frame 不进入 `Renderer` 或具体 GPU crate；
- host 只依赖 `Renderer`，具体 backend 类型只出现在 composition root；
- backend-specific 优化不能污染 scene API，除非该能力对所有后端都有稳定语义。

`component_composition_tests::ui_contract_does_not_depend_on_a_gpu_backend` 会扫描 `zeta-ui` manifest
和源码，阻止重新引入 `wgpu`；`gpu_backend_does_not_own_interaction_or_accessibility_frames` 阻止
input/accessibility ownership 漂入 backend。Rust 类型系统则保证 Native 的 renderer 字段只暴露 trait。

## 当前状态与演进

当前已完成 scene/backend 分离、ordered scene batching、presentation/interaction 与 GPU execution
分离、`Renderer` trait、wgpu adapter、Native trait-object ownership，以及 rect/image/icon/text
pipeline 从 `zeta-ui` 向 `zeta-wgpu` 的迁移。尚未实现 backend capability
negotiation、运行时热切换、raw Metal/Vulkan backend 或跨 backend golden-image 一致性测试。

新增 backend 时，应建立独立 crate、实现 `Renderer`、在 product composition root 替换 factory，
并针对 primitive、色彩空间、clip、layer、字体 fallback 与 surface recovery 建立一致性测试。
