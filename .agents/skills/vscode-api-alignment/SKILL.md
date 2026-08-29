---
name: vscode-api-alignment
description: Align Zeta TypeScript editor APIs with the checked-out VS Code source by preserving corresponding public names, responsibilities, lifecycle, and caller contracts. Use when adding, renaming, refactoring, or reviewing Zeta editor APIs against VS Code; do not use for Rust or generic TypeScript naming.
---

# VS Code API 对齐

当 Zeta 的 TypeScript 编辑器实现对应 VS Code 的模块、视图部件或公共契约时，使用 `../vscode` 作为源码参考，使用 `zeta-ts/src/zeta` 作为修改范围。目标是保持可验证的 API 对应关系，不是复制上游实现。

## 必须保持的契约

- 对应的导出类、接口、类型、函数、枚举、公共常量、配置项、回调属性和方法名必须保持 VS Code 的准确拼写与大小写；至少保证所有对外名称和配置键同名。
- 对应符号承担与 VS Code 相同的可观察职责、所有权、生命周期、事件响应、输入输出含义和失败语义。不要把多个职责合并到一个对外符号，也不要把一个职责拆成不同的公开名称。
- 本地基类、内部数据形状或事件机制可以不同，但差异必须留在内部实现或边界适配中，不能成为修改对外名称的理由。
- 不添加别名、兼容性垫片、临时名称、兜底分支或重复入口来掩盖偏差。若同名要求与现有调用方冲突，更新当前范围内的调用方、测试和文档；无法安全更新时，先报告证据，不要猜测。
- 只有在 VS Code 没有对应能力时，才允许使用 Zeta 专有名称，并明确它是 Zeta 专有能力，不要把它包装成上游 API。

例如，VS Code 导出 `LineNumbersOverlay` 时，本地对应类必须继续叫 `LineNumbersOverlay`；本地使用不同的视图基类，只是实现差异，不是改名理由。

## 工作流程

1. 先确定本地 owner、调用方、导出入口和测试范围，并读取将要修改文件的仓库及 scoped instructions。
2. 在 `../vscode` 中按符号和职责寻找对应实现，不要只按文件名匹配。至少检查导出项、直接调用方、基类职责、生命周期、事件处理、配置读取、渲染或输入副作用，以及邻近模块的边界。
3. 先建立一张精简的对应表，再修改代码：

   | VS Code 对应 API | Zeta API | 名称是否准确一致 | 职责与 owner | 生命周期、调用方和已确认差异 |
   | --- | --- | --- | --- | --- |

   对应 API 缺失时标记为缺失，不要先造一个猜测名称。表中的差异必须能在源码、调用方或测试中找到依据。
4. 让本地对外名称、参数含义、返回值、枚举成员、配置键和回调契约与对应 VS Code API 对齐；把 Zeta 的内部实现差异收敛在同一个 owner 内，保持 `base` → `platform` → `editor` → `workbench` 的依赖方向。
5. 同步更新本地 import、export、调用方、测试和当前文档。不要复制与当前职责无关的上游私有实现，也不要为了追求表面相似而新建没有调用方的抽象。

## 验证要求

- 用 `rg` 做仓库级搜索，确认旧名称、错误别名和重复入口不再被必需调用方引用，并确认对应的导出名称仍然可追踪。
- 运行覆盖本次修改的最小 TypeScript 类型检查和测试；命令失败时区分变更引入的问题与已有失败，不得把未运行或失败的检查写成通过。
- 如果改动影响浏览器或 Electron 的可见行为，使用 Playwright 验证真实交互；调试 UI 失败时保留必要截图作为证据。
- 最后复核对应表、`git diff` 和 `git status`，报告：已同名的 API、仍存在的职责差异、Zeta 专有 API、实际运行的检查，以及未解决的问题。
