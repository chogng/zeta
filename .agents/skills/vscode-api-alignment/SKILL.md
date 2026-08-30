---
name: vscode-api-alignment
description: Align Zeta TypeScript editor source files, APIs, and debuggable call paths with the checked-out VS Code source by preserving corresponding paths, file decomposition, public names, responsibilities, ownership, lifecycle, and caller contracts. Use when adding, renaming, refactoring, or reviewing Zeta editor APIs against VS Code; do not use for Rust or generic TypeScript naming.
---

# VS Code API 对齐

当 Zeta 的 TypeScript 编辑器实现对应 VS Code 的模块、视图部件或公共契约时，以 `../vscode` 为源码依据，以 `zeta-ts/src/zeta/editor` 及其必要调用方为主要修改范围。目标是建立可以沿调用链定位问题的真实对应关系，不是复制上游，也不是只让类型检查通过。

## 分层与依赖方向

遵循 `../vscode/.github/instructions/source-code-organization.instructions.md` 指向的 Source Code Organization：

- `base` 提供不依赖服务的通用能力和 UI 基础部件。
- `platform` 提供跨层共享的服务、服务注册与注入能力。
- `editor` 可以依赖 `base`、`platform` 和自身更低运行环境的模块。
- `workbench` 可以依赖 `editor`、`platform`、`base`，禁止反向依赖。
- `common` 只使用基础 JavaScript；DOM 只进入 `browser`。`editor` 不得依赖 Node 或 Electron 环境代码。

因此，editor 依赖 base/platform 本身是正常分层，不要为了“editor 自包含”复制这些能力。需要检查的是依赖方向、owner 和运行环境是否正确。

## 对齐判定

只有以下条件同时成立，才能把一个 API 标记为已对齐：

- 先在所选所有权切片内逐项比较生产源码文件集合。对应实现使用相同的相对文件和目录路径；文件数量及职责拆分是架构契约的一部分，不得在未审计文件集合时直接修改 API。
- 对外类、接口、类型、函数、枚举、常量、配置项、回调属性和方法使用 VS Code 的准确拼写与大小写。
- 符号来自对应的 owning module path。上游从 `common/core/range.js` 导入 `Range`，本地对应文件也必须从对应 owner 导入；不得从聚合文件、旧路径、import alias 或局部重复类型绕过。
- 文件和目录路径按大小写比较，并由 Git 正确记录。Windows 能解析错误大小写，不代表路径已对齐。
- 对应符号承担相同的可观察职责，包括状态 owner、调用顺序、失效条件、调度阶段、坐标转换、副作用、错误语义和释放时机。
- 上游一个行为链中的关键阶段，在 Zeta 都有职责明确的对应 owner，调用方实际经过这些 owner。

参数和返回类型一致只说明输入边界接近，不能证明实现效果一致。名称相同但职责、owner 或调用链不同属于假对齐。

## 禁止做法

- 不得添加别名、兼容垫片、临时名称、兜底分支、重复入口或聚合 re-export 掩盖差异。
- 不得把多个上游职责合并到一个同名公开符号，也不得把一个上游职责拆成无法追踪的多个公开名称。
- 不得把职责不同的本地文件直接改成上游文件名。缺少基础 owner 时，明确记录阻塞并先补齐真实能力。
- 不得创建没有生产调用方的空接口、空类或上游文件副本来满足结构检查。
- 不得新增仅本地的桥接、适配、事务或聚合生产文件来承载本应属于上游既有 owner 的职责。发现仅本地文件时，先追踪全部生产调用方和状态所有权，再把职责迁入准确的上游对应路径并删除旧文件。
- 只有 VS Code 没有对应能力时才使用明确的 Zeta 专有名称，并在状态文档中说明。

## 工作方式

1. 修改代码前先运行文件集合审计。若任务范围是整个 Editor，就审计整个 `zeta-ts/src/zeta/editor`；若范围较小，就审计完整的所有权切片而不是单个被点名文件。逐项记录同路径、仅本地、仅上游和大小写不一致，未分类完不得新建或重命名生产文件。
2. 从 `../vscode` 选定准确的对应文件，读取完整 import 列表、导出、直接调用方和测试；不要从本地现状猜上游结构。
3. 建立对应表，至少记录：文件集合状态、上游符号与路径、本地符号与路径、owner 是否一致、职责差异、调用链和生命周期差异。每个仅本地生产文件必须标为“迁移并删除”或“Zeta 专有”，后者必须说明其独立产品职责以及为什么不介入上游对应调用链。
4. 对每条不一致的 import 递归追溯 owner。上游依赖的基础文件或语义尚未存在时，先对齐依赖 owner，再返回当前文件。
5. 从一个真实用户行为、上游测试或 bug 路径出发，核对触发点、状态变化、副作用、坐标语义、异常路径和释放顺序。
6. 调整 owner 和实际调用链后，再统一公开名称、路径、参数、返回值、枚举、配置键及回调契约。
7. 同步修改所有必要调用方、测试和当前架构文档，迁移完调用方后删除错误文件和旧入口；不要保留两套路径。

当本地一个文件承担上游两个文件的职责时，应按上游 owner 拆回准确的相对路径，并迁移调用方、状态与生命周期；不得以“本地实现更紧凑”为理由维持不同的生产文件图。反过来，一个上游文件的职责也不得散落到多个仅本地文件。Zeta 专有能力必须拥有独立产品职责，并与对应上游调用链清楚分离。

当一个本地对象同时实现多个上游契约时，逐项比较同名方法的坐标域和输入语义。同名方法若分别表示逻辑模型、视觉投影或浏览器测量，就必须由独立 context 或 owner 承担；不得让一个实现用调用时机猜测语义。调用方应显式传入对应契约，并用跨坐标转换测试证明边界。

## 验证

- 修改前和每批迁移后运行 `node .agents/skills/vscode-api-alignment/scripts/audit-editor-file-set.mjs`；先消化本批触及所有权切片的仅本地、仅上游与大小写不一致项，再检查 API 成员。整个 Editor 对齐任务必须保留逐文件结果，不能只报告数量。
- 涉及 Editor 118 项对齐账目时，先运行 `node .agents/skills/vscode-api-alignment/scripts/verify-editor-api-ledger.mjs`，确认状态文档中的已处理数、待处理数和 118 个唯一声明一致；每完成一项就把它从待处理表移动到已处理表、同步摘要数字，再次运行脚本。
- 从 118 项账目选择下一批时，可运行 `node .agents/skills/vscode-api-alignment/scripts/compare-editor-api-members.mjs`，按同路径声明的成员名差异量排序。该结果只用于缩小人工核对范围；成员名相同仍必须继续核对 owner、行为和调用链。
- 用 `rg` 确认旧名称、错误大小写路径、别名、重复入口和聚合导入不再被生产代码引用。
- 对受影响文件逐项复核上游与本地 import 的符号和相对 owner path；只看导出声明或编译成功不算验证。
- 检查 `editor` 的层级和运行环境依赖，尤其防止 `common` 间接引入 DOM、Node 或 Electron 能力。
- 至少验证一条受影响的真实行为链，覆盖触发、owner、状态变化、副作用和释放。
- 运行最小 TypeScript 检查与相关测试。区分本次引入的失败和仓库已有失败，不得把未运行或失败的检查写成通过。
- 最后复核对应表、`git diff`、`git status`，报告已真实对齐、仍是假对齐或缺失的 owner、Zeta 专有 API、已运行检查和未解决问题。

## Learnings

* 对齐上游编辑器前，必须先比较整个目标所有权切片的相对生产源码文件集合；文件路径、数量与职责拆分都属于架构契约。任何仅本地生产文件必须先证明是明确的 Zeta 专有能力且不介入对应上游调用链，否则先迁移调用方并删除，不得用新增桥接文件承载上游 API。
