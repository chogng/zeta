# `zeta-renderer`

> 本 README 负责后端无关渲染契约的当前实现。跨 crate 的渲染所有权、替换路径与长期不变量见
> [`docs/rendering-architecture.md`](../docs/rendering-architecture.md)。`wgpu` 后端细节见
> [`zeta-wgpu`](../wgpu/README.md)，scene contract 见 [`zui`](../zui/README.md)。

`zeta-renderer` 是 `UiScene` 与具体图形 API 之间的稳定边界。产品 host 保存 `dyn Renderer`；
组件、布局和场景构造不接触 device、queue、command encoder、render pass 或 surface。

## 1. 所有权

| 能力 | 本 crate | 委托/不拥有 |
| --- | --- | --- |
| 后端无关的帧执行接口 | ✅ `Renderer` | |
| Physical target size、present outcome、统一错误边界 | ✅ | |
| Scene primitive、paint order 与 backend-neutral batch contract | ❌ | `zui` |
| Surface、GPU resource、shader、atlas 与提交 | ❌ | `zeta-wgpu` 或其他 backend crate |
| Window/event loop 与 redraw policy | ❌ | `zeta-winit` / product host |
| 后端选择和初始化 | ❌ | product composition root |

本 crate 禁止依赖 `wgpu`、Metal、Vulkan、DirectX、窗口系统或产品状态。出现具体图形 API 类型、
shader 或 surface handle，意味着 ownership 已经漂移。

## 2. 公共接口

| Symbol | 职责 | 实现义务 |
| --- | --- | --- |
| `Renderer` | 执行空帧或 immutable `UiScene` | 自行拥有并验证 backend/surface lifecycle，不修改 scene |
| `RenderTargetSize` | 明确传递 physical width/height | 零尺寸语义由 backend 处理 |
| `RenderOutcome` | 区分 `Presented`、`Skipped`、`Retry` | host 只在 `Retry` 后调度重绘 |
| `RendererError` | 擦除具体 backend error，同时保留 source chain | adapter 用 `RendererError::backend` 包装原始错误 |

调用路径：

```text
product component → UiScene
product host → Box<dyn Renderer>::render_scene
             → selected backend implementation
```

`Renderer` 是 object-safe 的，后端替换不要求 product state 或 component 泛型化。初始化不是 trait
职责，因为不同 backend 的 adapter、device selection 与 window handle 输入不同；composition root
负责创建实现后，把它擦除为 `Box<dyn Renderer>`。

## 3. 失败语义与测试

后端用 `Skipped` 表示当前无需 present，用 `Retry` 表示 surface 已恢复或重新配置、host 应请求
下一帧。不可恢复的 device、surface 或 scene prepare failure 进入 `RendererError`；本 crate 不决定
退出、降级或 backend migration。

```bash
cargo test --manifest-path Cargo.toml -p zeta-renderer
```

`renderer_tests::backend_can_be_replaced_without_changing_scene_production` 用 recording backend 验证
scene producer 只依赖 trait。真实 GPU 与 surface 验证属于具体 backend crate。

## 4. 修改影响与当前限制

- 新增所有 backend 都需要的帧语义时，修改 `Renderer`、所有实现、本 README 与系统文档；
- backend 私有的 instance packing、shader、pipeline merge 或 atlas 不得扩张本接口；跨 primitive
  的语义绘制顺序由 `zui::SceneBatch` 统一表达；
- 新 scene primitive 先进入 `zui`，每个 backend 再选择支持、降级或返回明确错误；
- 当前没有 backend capability negotiation、热切换或 device-lost 后自动迁移。
