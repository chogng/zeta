---
name: vscode-api-alignment
description: Align Zeta TypeScript editor APIs and debuggable call paths with the checked-out VS Code source by preserving corresponding public names, responsibilities, ownership, lifecycle, and caller contracts. Use when adding, renaming, refactoring, or reviewing Zeta editor APIs against VS Code; do not use for Rust or generic TypeScript naming.
---

# VS Code API 对齐

当 Zeta 的 TypeScript 编辑器实现对应 VS Code 的模块、视图部件或公共契约时，使用 `../vscode` 作为源码参考，使用 `zeta-ts/src/zeta` 作为修改范围。目标是保持可验证、可沿调用链排查问题的对应关系，不是复制上游实现。

架构分层以 `../vscode/.github/instructions/source-code-organization.instructions.md` 及其指向的 [Source Code Organization](https://github.com/microsoft/vscode/wiki/Source-Code-Organization) 为准：`base` 提供无服务依赖的通用能力，`platform` 提供跨层共享服务，`editor` 可以依赖 `base` 和 `platform`，`workbench` 可以依赖 `editor` 及更低层，反向依赖禁止。`common` 只使用基础 JavaScript，`browser` 才能使用 DOM；`editor` 不得依赖 `node` 或 `electron-*` 环境代码。

## 必须保持的契约

- 对应的导出类、接口、类型、函数、枚举、公共常量、配置项、回调属性和方法名必须保持 VS Code 的准确拼写与大小写；至少保证所有对外名称和配置键同名。
- 对应源码文件的 import 符号和 owning module path 也是架构契约。VS Code 从 `common/core/range.js` 导入 `Range` 时，Zeta 的对应文件必须从对应的 `common/core/range.js` 导入 `Range`；不得改从聚合模块导入、使用 import alias 伪装同名，或把该符号留在职责不同的文件中。
- 对应符号承担与 VS Code 相同的可观察职责、所有权、生命周期、事件响应、输入输出含义和失败语义。不要把多个职责合并到一个对外符号，也不要把一个职责拆成不同的公开名称。
- 禁止只改成同名却保留不同职责的假对齐。同名符号若不能承担上游对应职责，先调整 owner 和调用链；只有确认它是 Zeta 专有能力时，才使用明确的 Zeta 专有名称。
- 输入边界、参数类型或返回类型一致，不能证明实现效果一致。还必须核对状态 owner、事件顺序、失效条件、调度阶段、坐标转换、可见副作用、异常路径和释放时机。
- 从 VS Code 的一个用户行为、测试或 bug 调用路径出发，Zeta 在每个关键阶段都必须能找到一个职责明确的对应 owner。若一个上游阶段在 Zeta 被分散到多个无明确边界的对象，先收敛职责再宣布对齐完成。
- 本地基类、内部数据形状或事件机制可以不同，但差异必须留在内部实现或边界适配中，不能成为修改对外名称的理由。
- 不添加别名、兼容性垫片、临时名称、兜底分支或重复入口来掩盖偏差。若同名要求与现有调用方冲突，更新当前范围内的调用方、测试和文档；无法安全更新时，先报告证据，不要猜测。
- 只有在 VS Code 没有对应能力时，才允许使用 Zeta 专有名称，并明确它是 Zeta 专有能力，不要把它包装成上游 API。

例如，VS Code 导出 `LineNumbersOverlay` 时，本地对应类必须继续叫 `LineNumbersOverlay`；本地使用不同的视图基类，只是实现差异，不是改名理由。

## 工作流程

1. 从 `../vscode` 中选定准确的上游文件作为起点，再确定本地对应文件、调用方、导出入口和测试范围，并读取将要修改文件的仓库及 scoped instructions。不要先从本地现状推测上游结构。
2. 完整列出上游文件的 imports，把它们同时当作能力清单和模块所有权图，逐项核对 import specifier、相对路径、导出 owner 和输入输出语义。
3. 沿每一条不一致的 import 递归追溯：如果上游依赖的符号、文件或基础语义在 Zeta 尚未对齐，先把该依赖 owner 对齐，再返回当前文件。不得在下游使用聚合 re-export、import alias、局部包装类或重复类型绕过未对齐的基础模块。
4. 递归追溯至少覆盖导出项、直接调用方、基类职责、状态 owner、生命周期、事件顺序、配置读取、失效与调度、坐标转换、渲染或输入副作用、失败语义，以及邻近模块的边界。发现坐标基数、范围闭合规则或 selection 方向等基础语义不同时，当前文件标记为阻塞，不得宣布对齐。
5. 先建立一张精简的对应表，再修改代码：

   | VS Code import（符号与路径） | Zeta import（符号与路径） | 名称和 module owner 是否一致 | 职责与 owner | 触发、调用链、副作用与生命周期差异 |
   | --- | --- | --- | --- | --- |

   对应 API 缺失时标记为缺失，不要先造一个猜测名称。表中的差异必须能在源码、调用方或测试中找到依据。名称一致但 owner、调用顺序或副作用不一致时，标记为假对齐，不得写成已完成。
6. 先让基础依赖、职责、owner、调用链和生命周期对应，再让本地对外名称、参数含义、返回值、枚举成员、配置键和回调契约与对应 VS Code API 同名。把 Zeta 的内部实现差异收敛在对应 owner 内，并按 Source Code Organization 检查层级和运行环境依赖。
7. 同步更新本地 import、export、调用方、测试和当前文档。对应模块不得继续通过聚合 re-export、旧路径或 import alias 暴露已经迁移的 API；移除旧入口并更新所有调用方。不要复制与当前职责无关的上游私有实现，也不要为了追求表面相似而新建没有调用方的抽象。

## 验证要求

- 用 `rg` 做仓库级搜索，确认旧名称、错误别名和重复入口不再被必需调用方引用，并确认对应的导出名称仍然可追踪。
- 对受影响的每个对应文件比较 import 列表，确认上游已有对应模块的符号名和相对 owning path 一致；只检查导出声明或编译通过不算完成。
- 检查 `editor` 只向 `base`、`platform` 和自身的更低运行环境依赖；`common` 不得通过间接 import 引入 `browser`、DOM、Node 或 Electron 能力。
- 选取至少一条受影响的真实行为路径，逐段复核 VS Code 与 Zeta 的触发点、owner、状态变化、副作用和释放顺序；名称搜索不能替代这项检查。
- 运行覆盖本次修改的最小 TypeScript 类型检查和测试；命令失败时区分变更引入的问题与已有失败，不得把未运行或失败的检查写成通过。
- 如果改动影响浏览器或 Electron 的可见行为，使用 Playwright 验证真实交互；调试 UI 失败时保留必要截图作为证据。
- 最后复核对应表、真实行为路径、`git diff` 和 `git status`，报告：已同名且职责对应的 API、仍存在的假对齐或职责差异、Zeta 专有 API、实际运行的检查，以及未解决的问题。
