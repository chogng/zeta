# Editor 对齐

仅在目标位于 `zeta-ts/src/zeta/editor` 时读取。共同的单一实现、删除确认和验证规则由主 `SKILL.md` 负责；Editor 在此基础上执行整层严格文件图收敛。

## 对应范围

- 本地根目录：`zeta-ts/src/zeta/editor`
- 上游根目录：`../vscode/src/vs/editor`
- 目标：生产 TypeScript 文件数量、相对路径、文件名和大小写一致；对应文件承担相同公开契约、职责、状态所有权、生命周期和调用链。
- `editor` 可以依赖 `base`、`platform` 和自身更低运行环境的模块，不为“自包含”复制这些能力。`common` 只使用基础 JavaScript，DOM 进入 `browser`。

## 批量调查示例

用户指定 Editor 目录或同一目录下多个文件时，首次调查直接批量取得该目录双方文件集合、imports、exports、生产调用方和测试。输出过大时按结果维度拆分，不逐文件暂停。

修改前保存工作树来源基线，并对整个目标所有权切片分类同路径、仅本地、仅上游和大小写差异。每批修改后重新审计；仅本地生产文件只能减少，新增生产文件必须属于修改前的仅上游集合。

## 专用检查

默认运行：

```powershell
node .agents/skills/vscode-api-alignment/scripts/check-editor-alignment.mjs
```

该入口依次执行：

- `verify-editor-api-ledger.mjs`：验证 118 项台账的唯一声明和已处理/待处理数量；
- `audit-editor-file-set.mjs`：审计整个 Editor 生产文件集合；
- `compare-editor-api-members.mjs`：按台账中的同名声明报告成员名差异；
- `git diff --check`；
- 仓库 `typecheck:stanza`；
- 检查运行前后是否新增未跟踪 `.js`。

需要完整逐文件输出时使用 `--full`。仅做修改前结构调查时可以使用 `--structure-only`；代码修改后的最终验证不能用它代替类型检查。

成员名报告只用于缩小人工核对范围。差异为零仍需检查签名、可见性、继承、owner、行为和调用链；缺失文件或声明也不能按数字零处理。

## TypeScript 输出边界

`zeta-ts/tsconfig.renderer.json` 必须保持 `noEmit: true`，Stanza 与扩展检查配置继承该不变量。只有负责生成产物的构建或测试配置可以启用输出，并必须把 `outDir` 明确设置在源码树外。

## 完成判定

一项 Editor API 只有在下列事实同时成立后才能从待处理表移入已处理表：

- 文件和 owning module 对应；
- 公开名称、签名和调用方对应；
- 状态、失效、调度、坐标、副作用、错误和释放职责对应；
- 生产调用真实经过对应 owner；
- 本地独立行为测试通过；
- 专用检查入口通过。

删除仅本地文件时仍执行主 skill 的逐路径确认；调用方归零或上游不存在该文件都不能单独证明职责已完成迁移。
