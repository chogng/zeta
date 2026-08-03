# Desktop 测试结构

本目录只保存跨源码 owner 的测试基础设施和完整应用测试。单元与组件测试跟随实现放在 `src/zeta/<owner>/test/<runtime>`。

## 快速理解

| 测试类型 | 放置位置 | 运行入口 |
| --- | --- | --- |
| 单元与组件契约 | `src/zeta/<owner>/test/common|browser|node|electron-*` | `pnpm test:unit` |
| 仓库级架构约束 | `test/architecture` | `pnpm test:unit` |
| Electron 自动化驱动 | `test/automation` | 由 smoke tests 引用 |
| 快速 Renderer 场景 | `test/smoke/areas/<area>` | `pnpm test:smoke:ui` |
| 完整 Desktop 场景 | `test/smoke/areas/<area>` | `pnpm test:smoke:desktop` |
| 构建脚本测试 | `scripts/*.test.mjs` | `pnpm test:scripts` |

`pnpm test:main` 依次运行构建脚本测试和全部单元测试。`pnpm test:smoke:ui` 启动禁用 App Server 的 Electron，适合快速验证 Renderer 和 Workbench；`pnpm test:smoke:desktop` 会额外组装 Rust 开发包并启动真实 App Server。默认的 `pnpm test:smoke` 和仓库根目录 `pnpm test:desktop:smoke` 均指向完整 Desktop 模式；根目录 `pnpm test:desktop:smoke:ui` 显式运行快速模式。

新增测试时先选择拥有被验证 contract 的最窄源码模块，再按真实运行时选择 `common`、`browser`、`node`、`electron-browser` 或 `electron-main`。只有没有单一源码 owner 的全仓库约束才进入 `test/architecture`；跨多个用户操作的场景才进入 `test/smoke`。
