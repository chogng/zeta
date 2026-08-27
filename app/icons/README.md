# `zeta-icons`

> 本 README 拥有 Rust 产品图标 API、生成绑定和接入规则。跨客户端资源规范见 [`docs/icons.md`](../../docs/icons.md)，SVG 操作入口见 [`resources/README.md`](../../resources/README.md)。

`zeta-icons` 提供从 `resources/icons/*.svg` 自动生成的稳定图标常量。它不拥有布局、主题颜色、GPU 图集、栅格化或组件。

## 1. 公共契约

| 符号 | 责任 |
| --- | --- |
| `icons::*` | 按 SVG 文件名生成的类型化 `Icon` 常量 |
| `ALL_ICONS` | 按 ID 排序的完整图标目录 |
| `icon_by_id` | 二分查找已生成的图标 ID |
| `IconRendering` | 区分随调用方着色的单色图标和保留固定颜色的多彩图标 |

`Icon`、`IconId`、`IconDefinition` 和 `IconRendering` 的通用资源契约由 `zui` 拥有，`zeta-icons` 只提供产品资源目录。

## 2. 生成路径

```text
resources/icons/*.svg
  → build/resources/icons/generate.ts
    → generate-to-rs.ts
  → resources/icons/manifest.json
  → app/icons/src/generated.rs
  → icons::* / ALL_ICONS
```

`generated.rs` 同时包含私有 SVG 字节绑定、公共常量、排序目录和测试目录。图标 ID、Rust 常量名和 `IconRendering` 都由同一次 SVG 扫描生成，不存在手写 `ID → artwork` 映射。

## 3. 修改路径

在仓库根目录添加或修改 SVG 后运行：

```bash
pnpm icons:generate
pnpm icons:check
cargo test --manifest-path Cargo.toml -p zeta-icons
```

生成器拒绝不规范文件名、多 SVG 根节点、脚本、`foreignObject`、事件处理器和外部链接。生成文件会提交到仓库，Cargo 和 Bazel 编译动作只读取产物，不运行 Node.js 或改写源码。

## 4. 漂移信号

- 出现手写 `IconId::new` 产品常量，表示图标目录绕过了生成器。
- 产品代码通过路径或 `include_bytes!` 直接读取 `resources/icons`，表示资源边界被绕过。
- Rust 代码自行判断颜色值，表示渲染模式与 manifest 分叉。
- 新增另一份产品 SVG 目录，表示唯一资源源被破坏。
