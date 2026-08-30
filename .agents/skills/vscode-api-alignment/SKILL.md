---
name: vscode-api-alignment
description: Align Zeta TypeScript editor APIs and debuggable call paths with the checked-out VS Code source by preserving corresponding public names, responsibilities, ownership, lifecycle, and caller contracts. Use when adding, renaming, refactoring, or reviewing Zeta editor APIs against VS Code; do not use for Rust or generic TypeScript naming.
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
- 只有 VS Code 没有对应能力时才使用明确的 Zeta 专有名称，并在状态文档中说明。

## 工作方式

1. 先从 `../vscode` 选定准确的对应文件，读取完整 import 列表、导出、直接调用方和测试；不要从本地现状猜上游结构。
2. 建立简短对应表，至少记录：上游符号与路径、本地符号与路径、owner 是否一致、职责差异、调用链和生命周期差异。
3. 对每条不一致的 import 递归追溯 owner。上游依赖的基础文件或语义尚未存在时，先对齐依赖 owner，再返回当前文件。
4. 从一个真实用户行为、上游测试或 bug 路径出发，核对触发点、状态变化、副作用、坐标语义、异常路径和释放顺序。
5. 调整 owner 和实际调用链后，再统一公开名称、路径、参数、返回值、枚举、配置键及回调契约。
6. 同步修改所有必要调用方、测试和当前架构文档，并删除旧入口；不要保留两套路径。

当本地一个文件承担上游两个文件的职责时，先判断这两个上游 owner 是否代表独立状态、生命周期或可替换能力。若是，就说明本地解耦不足，应按职责拆开；若只是本地内部实现更紧凑且对外 owner、调用链和行为仍一一可追踪，则不为文件数量机械拆分。

## 验证

- 用 `rg` 确认旧名称、错误大小写路径、别名、重复入口和聚合导入不再被生产代码引用。
- 对受影响文件逐项复核上游与本地 import 的符号和相对 owner path；只看导出声明或编译成功不算验证。
- 检查 `editor` 的层级和运行环境依赖，尤其防止 `common` 间接引入 DOM、Node 或 Electron 能力。
- 至少验证一条受影响的真实行为链，覆盖触发、owner、状态变化、副作用和释放。
- 运行最小 TypeScript 检查与相关测试。区分本次引入的失败和仓库已有失败，不得把未运行或失败的检查写成通过。
- 最后复核对应表、`git diff`、`git status`，报告已真实对齐、仍是假对齐或缺失的 owner、Zeta 专有 API、已运行检查和未解决问题。
