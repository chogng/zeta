# Desktop 测试结构

本目录只保存跨源码 owner 的测试基础设施和完整应用测试。单元与组件测试跟随实现放在 `src/zeta/<owner>/test/<runtime>`。

## 快速理解

| 测试类型 | 放置位置 | 运行入口 |
| --- | --- | --- |
| 单元与组件契约 | `src/zeta/<owner>/test/common|browser|node|electron-*` | `pnpm test:unit` |
| 仓库级架构约束 | `test/architecture` | `pnpm test:unit` |
| Electron 自动化驱动 | `test/automation` | 由 smoke tests 引用 |
| 完整应用场景 | `test/smoke/areas/<area>` | `pnpm test:smoke` |
| 构建脚本测试 | `scripts/*.test.mjs` | `pnpm test:scripts` |

`pnpm test:main` 依次运行构建脚本测试和全部单元测试。仓库根目录的 `pnpm test:desktop:smoke` 会构建 Desktop、检查自动化 TypeScript，并运行真实 Electron 场景。

新增测试时先选择拥有被验证 contract 的最窄源码模块，再按真实运行时选择 `common`、`browser`、`node`、`electron-browser` 或 `electron-main`。只有没有单一源码 owner 的全仓库约束才进入 `test/architecture`；跨多个用户操作的场景才进入 `test/smoke`。
